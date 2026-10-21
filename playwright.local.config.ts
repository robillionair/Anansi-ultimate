import base from './playwright.config';
export default {
  ...base,
  use: {
    ...(base as { use?: object }).use,
    launchOptions: {
      executablePath: '/home/hatch/workspace/anansi-promo/node_modules/.remotion/chrome-headless-shell/linux64/chrome-headless-shell-linux64/chrome-headless-shell',
      args: ['--no-sandbox', '--disable-dev-shm-usage'],
    },
  },
};
