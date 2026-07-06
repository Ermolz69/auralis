import { invoke } from '@/shared/api/tauri';

export async function healthCheck() {
  return invoke<string>('health_check');
}
