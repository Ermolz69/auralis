import { PasteYoutubeLink } from '../../../features/paste-youtube-link';

export const HomePage = () => {
  return (
    <div className="min-h-screen bg-bg flex flex-col items-center justify-center p-8">
      <div className="w-full max-w-2xl text-center flex flex-col gap-8">
        <h1 className="text-5xl font-bold text-text bg-gradient-to-r from-primary to-danger bg-clip-text text-transparent pb-2">Auralis</h1>
        <p className="text-muted text-xl">AI-powered video dubbing straight from your desktop.</p>
        <div className="mt-4">
          <PasteYoutubeLink />
        </div>
      </div>
    </div>
  );
};
