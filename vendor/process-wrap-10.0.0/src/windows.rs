//! Windows API support functions.

use std::{
	collections::HashMap,
	io::{Error, Result},
	os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle as NativeProcessHandle},
};

#[cfg(feature = "tracing")]
use tracing::{debug, instrument};
use windows::{
	Win32::{
		Foundation::{
			CloseHandle, ERROR_INVALID_PARAMETER, ERROR_MORE_DATA, ERROR_NO_MORE_FILES, HANDLE,
			INVALID_HANDLE_VALUE, WAIT_OBJECT_0, WAIT_TIMEOUT,
		},
		System::{
			Diagnostics::ToolHelp::{
				CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First,
				Thread32Next,
			},
			IO::{CreateIoCompletionPort, GetQueuedCompletionStatus, OVERLAPPED},
			JobObjects::{
				AssignProcessToJobObject, CreateJobObjectW, IsProcessInJob,
				JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOBOBJECT_ASSOCIATE_COMPLETION_PORT,
				JOBOBJECT_BASIC_ACCOUNTING_INFORMATION, JOBOBJECT_BASIC_PROCESS_ID_LIST,
				JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectAssociateCompletionPortInformation,
				JobObjectBasicAccountingInformation, JobObjectBasicProcessIdList,
				JobObjectExtendedLimitInformation, QueryInformationJobObject,
				SetInformationJobObject, TerminateJobObject,
			},
			SystemServices::JOB_OBJECT_MSG_NEW_PROCESS,
			Threading::{
				CREATE_SUSPENDED, GetProcessId, OpenProcess, OpenThread, PROCESS_CREATION_FLAGS,
				PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, ResumeThread,
				THREAD_SUSPEND_RESUME, WaitForSingleObject,
			},
		},
	},
	core::HRESULT,
};

#[cfg(feature = "std")]
use std::{ops::ControlFlow, time::Duration};
#[cfg(feature = "std")]
use windows::Win32::System::Threading::INFINITE;

#[derive(Debug)]
struct OwnedHandle(HANDLE);

impl OwnedHandle {
	fn into_raw(self) -> HANDLE {
		let handle = self.0;
		std::mem::forget(self);
		handle
	}
}

impl Drop for OwnedHandle {
	fn drop(&mut self) {
		unsafe { CloseHandle(self.0) }.ok();
	}
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct JobCreationFlags {
	pub flags: PROCESS_CREATION_FLAGS,
	pub resume_after_assignment: bool,
}

pub(crate) fn job_creation_flags(user_flags: PROCESS_CREATION_FLAGS) -> JobCreationFlags {
	JobCreationFlags {
		flags: user_flags | CREATE_SUSPENDED,
		resume_after_assignment: !user_flags.contains(CREATE_SUSPENDED),
	}
}

#[cfg(test)]
mod creation_flag_tests {
	use windows::Win32::System::Threading::{CREATE_NEW_PROCESS_GROUP, CREATE_NO_WINDOW};

	use super::*;

	#[test]
	fn adds_suspension_without_losing_user_flags() {
		let user_flags = CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW;
		let policy = job_creation_flags(user_flags);
		assert_eq!(policy.flags, user_flags | CREATE_SUSPENDED);
		assert!(policy.resume_after_assignment);
	}

	#[test]
	fn preserves_explicit_suspension() {
		let user_flags = CREATE_NO_WINDOW | CREATE_SUSPENDED;
		let policy = job_creation_flags(user_flags);
		assert_eq!(policy.flags, user_flags);
		assert!(!policy.resume_after_assignment);
	}
}

#[derive(Clone, Copy, Debug)]
pub struct JobHandle(pub HANDLE);

unsafe impl Send for JobHandle {}
unsafe impl Sync for JobHandle {}

#[derive(Clone, Copy, Debug)]
pub struct PortHandle(pub HANDLE);

unsafe impl Send for PortHandle {}
unsafe impl Sync for PortHandle {}

/// A JobObject and its associated completion port.
///
/// This struct closes the handles when dropped.
#[derive(Debug)]
pub(crate) struct JobPort {
	pub job: JobHandle,
	pub completion_port: PortHandle,
}

impl Drop for JobPort {
	fn drop(&mut self) {
		unsafe { CloseHandle(self.job.0) }.ok();
		unsafe { CloseHandle(self.completion_port.0) }.ok();
	}
}

/// Create a JobObject and an associated completion port.
///
/// If `kill_on_drop` is true, we opt into the `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` flag, which
/// essentially implements the "reap children" feature of Unix systems directly in Win32.
#[cfg_attr(feature = "tracing", instrument(level = "debug"))]
pub(crate) fn make_job_object(process_handle: HANDLE, kill_on_drop: bool) -> Result<JobPort> {
	let job = OwnedHandle(unsafe { CreateJobObjectW(None, None) }.map_err(Error::other)?);
	#[cfg(feature = "tracing")]
	debug!(?job, "done CreateJobObjectW");

	let completion_port =
		OwnedHandle(unsafe { CreateIoCompletionPort(INVALID_HANDLE_VALUE, None, 0, 1) }?);
	#[cfg(feature = "tracing")]
	debug!(?completion_port, "done CreateIoCompletionPort");

	let associate_completion = JOBOBJECT_ASSOCIATE_COMPLETION_PORT {
		CompletionKey: job.0.0 as _,
		CompletionPort: completion_port.0,
	};

	unsafe {
		SetInformationJobObject(
			job.0,
			JobObjectAssociateCompletionPortInformation,
			(&associate_completion) as *const _ as _,
			std::mem::size_of_val(&associate_completion)
				.try_into()
				.expect("cannot safely cast to DWORD"),
		)
	}?;
	#[cfg(feature = "tracing")]
	debug!(
		?associate_completion,
		"done SetInformationJobObject(completion)"
	);

	let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();

	if kill_on_drop {
		info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
	}

	unsafe {
		SetInformationJobObject(
			job.0,
			JobObjectExtendedLimitInformation,
			&info as *const _ as _,
			std::mem::size_of_val(&info)
				.try_into()
				.expect("cannot safely cast to DWORD"),
		)
	}?;
	#[cfg(feature = "tracing")]
	debug!(?info, "done SetInformationJobObject(limit)");

	unsafe { AssignProcessToJobObject(job.0, process_handle) }?;
	#[cfg(feature = "tracing")]
	debug!(?job, ?process_handle, "done AssignProcessToJobObject");

	Ok(JobPort {
		job: JobHandle(job.into_raw()),
		completion_port: PortHandle(completion_port.into_raw()),
	})
}

/// Resume all threads in the process (ie resume the process).
///
/// This is a pretty terrible hack, but it's either this or we
/// re-implement all of Rust's std::process just to get access!
#[cfg_attr(feature = "tracing", instrument(level = "debug"))]
pub(crate) fn resume_threads(child_process: HANDLE) -> Result<()> {
	#[inline]
	unsafe fn inner(pid: u32, tool_handle: HANDLE) -> Result<()> {
		let mut entry = THREADENTRY32 {
			dwSize: std::mem::size_of::<THREADENTRY32>()
				.try_into()
				.expect("THREADENTRY32 is guaranteed to fit in a DWORD"),
			..Default::default()
		};
		unsafe { Thread32First(tool_handle, &mut entry) }.map_err(Error::other)?;

		let mut resumed = false;
		loop {
			if entry.th32OwnerProcessID == pid {
				let thread_handle = OwnedHandle(unsafe {
					OpenThread(THREAD_SUSPEND_RESUME, false, entry.th32ThreadID)
				}?);
				let previous_count = unsafe { ResumeThread(thread_handle.0) };
				if previous_count == u32::MAX {
					return Err(Error::last_os_error());
				}
				if previous_count > 0 {
					resumed = true;
				}
			}

			match unsafe { Thread32Next(tool_handle, &mut entry) } {
				Ok(()) => {}
				Err(error) if error.code() == HRESULT::from_win32(ERROR_NO_MORE_FILES.0) => {
					break;
				}
				Err(error) => return Err(Error::other(error)),
			}
		}

		if resumed {
			Ok(())
		} else {
			Err(Error::other("no thread belonging to the child was found"))
		}
	}

	let child_id = unsafe { GetProcessId(child_process) };
	if child_id == 0 {
		return Err(Error::last_os_error());
	}

	let tool_handle = OwnedHandle(unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) }?);
	unsafe { inner(child_id, tool_handle.0) }
}

/// Terminate a job object without waiting for the processes to exit.
#[cfg_attr(feature = "tracing", instrument(level = "debug"))]
pub(crate) fn terminate_job(job: JobHandle, exit_code: u32) -> Result<()> {
	unsafe { TerminateJobObject(job.0, exit_code) }.map_err(Error::other)
}

/// Query the retained job handle; parent exit and completion-port packets are not tree completion.
fn job_accounting(job: JobHandle) -> Result<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION> {
	let mut information = JOBOBJECT_BASIC_ACCOUNTING_INFORMATION::default();
	// The buffer has the exact Win32 accounting structure size and remains writable for the call.
	unsafe {
		QueryInformationJobObject(
			Some(job.0),
			JobObjectBasicAccountingInformation,
			std::ptr::from_mut(&mut information).cast(),
			std::mem::size_of_val(&information) as u32,
			None,
		)
	}
	.map_err(Error::other)?;
	Ok(information)
}

const MAX_OBSERVED_PROCESSES: usize = 65_536;

fn job_process_ids(job: JobHandle) -> Result<Vec<u32>> {
	let offset = std::mem::offset_of!(JOBOBJECT_BASIC_PROCESS_ID_LIST, ProcessIdList);
	let mut capacity = 32;
	loop {
		let bytes = offset + capacity * std::mem::size_of::<usize>();
		let mut buffer = vec![0_usize; bytes.div_ceil(std::mem::size_of::<usize>())];
		let result = unsafe {
			QueryInformationJobObject(
				Some(job.0),
				JobObjectBasicProcessIdList,
				buffer.as_mut_ptr().cast(),
				bytes as u32,
				None,
			)
		};
		match result {
			Ok(()) => {
				let header = unsafe { &*buffer.as_ptr().cast::<JOBOBJECT_BASIC_PROCESS_ID_LIST>() };
				let count = header.NumberOfProcessIdsInList as usize;
				if count > capacity {
					return Err(Error::other("Job process inventory exceeded its buffer"));
				}
				let ids = unsafe {
					std::slice::from_raw_parts(
						std::ptr::addr_of!(header.ProcessIdList).cast::<usize>(),
						count,
					)
				};
				return ids
					.iter()
					.map(|id| u32::try_from(*id).map_err(Error::other))
					.collect();
			}
			Err(error)
				if error.code() == HRESULT::from_win32(ERROR_MORE_DATA.0)
					&& capacity < MAX_OBSERVED_PROCESSES =>
			{
				capacity *= 2
			}
			Err(error) => return Err(Error::other(error)),
		}
	}
}

/// Native handles survive wait cancellation and process rundown. Accounting also
/// detects missing notifications, which must never be accepted as complete.
#[derive(Debug, Default)]
pub(crate) struct JobObservation {
	processes: HashMap<u32, Option<NativeProcessHandle>>,
	uncertainty: Option<String>,
}

impl JobObservation {
	pub fn remember_root(&mut self, pid: u32, handle: NativeProcessHandle) {
		self.processes.entry(pid).or_insert(Some(handle));
	}

	pub fn retain_failure(&mut self, error: &Error) {
		self.uncertainty.get_or_insert_with(|| error.to_string());
	}

	fn remember_pid(&mut self, pid: u32, job: JobHandle) -> Result<()> {
		if self.processes.contains_key(&pid) {
			return Ok(());
		}
		if pid == 0 || self.processes.len() >= MAX_OBSERVED_PROCESSES {
			return Err(Error::other(
				"Job process inventory exceeds inspection budget",
			));
		}
		let handle = match unsafe {
			OpenProcess(
				PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION,
				false,
				pid,
			)
		} {
			Ok(handle) => {
				let owned = unsafe { NativeProcessHandle::from_raw_handle(handle.0) };
				let mut assigned = windows::core::BOOL::default();
				unsafe { IsProcessInJob(handle, Some(job.0), &mut assigned) }
					.map_err(Error::other)?;
				if assigned.as_bool() {
					Some(owned)
				} else if unsafe { WaitForSingleObject(handle, 0) } == WAIT_OBJECT_0 {
					// An exited assignment, or a PID reused after that assignment exited.
					// Do not retain or control a process outside the exact job.
					None
				} else {
					return Err(Error::other(
						"Observed live PID is not a member of the retained job",
					));
				}
			}
			// A PID no longer backed by any process object has completed rundown.
			Err(error) if error.code() == HRESULT::from_win32(ERROR_INVALID_PARAMETER.0) => None,
			Err(error) => return Err(Error::other(error)),
		};
		self.processes.insert(pid, handle);
		Ok(())
	}

	pub fn complete(&mut self, port: &JobPort) -> Result<bool> {
		if let Some(error) = &self.uncertainty {
			return Err(Error::other(format!(
				"Job inspection remains unconfirmed: {error}"
			)));
		}
		for pid in job_process_ids(port.job)? {
			self.remember_pid(pid, port.job)?;
		}
		let mut drained = false;
		for _ in 0..MAX_OBSERVED_PROCESSES {
			let (mut code, mut key) = (0_u32, 0_usize);
			let mut packet = std::ptr::null_mut::<OVERLAPPED>();
			match unsafe {
				GetQueuedCompletionStatus(
					port.completion_port.0,
					&mut code,
					&mut key,
					&mut packet,
					0,
				)
			} {
				Ok(()) => {
					if key != port.job.0.0 as usize {
						return Err(Error::other("Unexpected job completion key"));
					}
					if code == JOB_OBJECT_MSG_NEW_PROCESS {
						self.remember_pid(
							u32::try_from(packet as usize).map_err(Error::other)?,
							port.job,
						)?;
					}
				}
				Err(error) if error.code() == HRESULT::from_win32(WAIT_TIMEOUT.0) => {
					drained = true;
					break;
				}
				Err(error) => return Err(Error::other(error)),
			}
		}
		if !drained {
			return Err(Error::other(
				"Job completion inventory exceeds inspection budget",
			));
		}
		let accounting = job_accounting(port.job)?;
		if accounting.ActiveProcesses != 0
			|| accounting.TotalProcesses as usize != self.processes.len()
		{
			return Ok(false);
		}
		for handle in self.processes.values().flatten() {
			match unsafe { WaitForSingleObject(HANDLE(handle.as_raw_handle()), 0) } {
				WAIT_OBJECT_0 => {}
				WAIT_TIMEOUT => return Ok(false),
				_ => return Err(Error::last_os_error()),
			}
		}
		Ok(true)
	}
}

#[cfg(test)]
mod job_query_tests {
	use super::*;
	#[test]
	fn invalid_job_query_fails_closed() {
		assert!(job_accounting(JobHandle(INVALID_HANDLE_VALUE)).is_err());
	}
}

/// Wait for a job to complete.
#[cfg_attr(feature = "tracing", instrument(level = "debug"))]
#[cfg(feature = "std")]
pub(crate) fn wait_on_job(
	completion_port: PortHandle,
	timeout: Option<Duration>,
) -> Result<ControlFlow<()>> {
	let mut code: u32 = 0;
	let mut key: usize = 0;
	let mut overlapped = OVERLAPPED::default();
	let mut lp_overlapped = &mut overlapped as *mut OVERLAPPED;

	let result = unsafe {
		GetQueuedCompletionStatus(
			completion_port.0,
			&mut code,
			&mut key,
			&mut lp_overlapped as *mut _,
			timeout.map_or(INFINITE, |d| d.as_millis().try_into().unwrap_or(INFINITE)),
		)
	};

	// ignore timing out errors unless the timeout was specified to INFINITE
	// https://docs.microsoft.com/en-us/windows/win32/api/ioapiset/nf-ioapiset-getqueuedcompletionstatus
	if timeout.is_some() && result.is_err() && lp_overlapped.is_null() {
		return Ok(ControlFlow::Continue(()));
	}

	Ok(ControlFlow::Break(()))
}
