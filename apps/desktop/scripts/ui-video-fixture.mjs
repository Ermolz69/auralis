export function installUiVideoFixture() {
  const invoke = window.__TAURI_INTERNALS__.invoke;
  const convert = window.__TAURI_INTERNALS__.convertFileSrc;
  let videoUrl;
  const createSample = async () => {
    const canvas = document.createElement('canvas');
    canvas.width = 320;
    canvas.height = 180;
    const context = canvas.getContext('2d');
    const stream = canvas.captureStream(10);
    const recorder = new MediaRecorder(stream, { mimeType: 'video/webm;codecs=vp8' });
    const chunks = [];
    recorder.ondataavailable = (event) => {
      if (event.data.size) chunks.push(event.data);
    };
    const finished = new Promise((resolve, reject) => {
      recorder.onstop = resolve;
      recorder.onerror = reject;
    });
    let frame = 0;
    const draw = () => {
      context.fillStyle = frame++ % 2 ? '#315377' : '#376338';
      context.fillRect(0, 0, 320, 180);
    };
    draw();
    recorder.start();
    const timer = setInterval(draw, 100);
    await new Promise((resolve) => setTimeout(resolve, 2000));
    recorder.stop();
    await finished;
    clearInterval(timer);
    stream.getTracks().forEach((track) => track.stop());
    return URL.createObjectURL(new Blob(chunks, { type: 'video/webm' }));
  };
  window.__TAURI_INTERNALS__.convertFileSrc = (file, ...args) =>
    file === 'fixture-video.webm' ? videoUrl : convert(file, ...args);
  window.__TAURI_INTERNALS__.invoke = async (command, args = {}) => {
    if (command === 'resolve_artifact_path_cmd' && args.artifactId.startsWith('fixture-video-')) {
      videoUrl ??= await createSample();
      window.__ui.calls.push({ command, args });
      return 'fixture-video.webm';
    }
    return invoke(command, args);
  };
}
