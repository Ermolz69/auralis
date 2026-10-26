import { Blocked } from './core.mjs';

export function uiFailure(message, operation = 'Application') {
  const text = `${operation}: ${message}`;
  if (
    /hash|digest|corrupt|integrity|unexpected|conflict|invalid|operation not supported/i.test(
      message,
    )
  )
    return new Error(text);
  if (
    /not configured|not installed|unavailable|install the streaming TTS package|HTTP (?:Error[: ]*)?(?:403|404|429|5\d\d)|timed? out.*(?:network|connect)|network|connection refused|ENOTFOUND/i.test(
      message,
    )
  )
    return new Blocked(text);
  return new Error(text);
}
