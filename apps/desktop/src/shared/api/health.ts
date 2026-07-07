import { invoke } from '@/shared/api/tauri';

export async function healthCheck() {
  return invoke('health_check');
}
