import { invoke } from '@tauri-apps/api/core';

export async function healthCheck() {
  return invoke<string>('health_check');
}
