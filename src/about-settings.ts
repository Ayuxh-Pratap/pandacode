import { preview } from './bridge';
import { icon } from './dom';
import { LOCAL_BUILD_UPDATES } from './app-updates';

/** Public project links, opened only when the user clicks. This build comes from a fork of
 * PandaCode, which the license asks it to credit. */
export const PROJECT_LINKS = {
  repository: 'https://github.com/Ayuxh-Pratap/pandacode',
  issues: 'https://github.com/Ayuxh-Pratap/pandacode/issues',
  license: 'https://github.com/Ayuxh-Pratap/pandacode/blob/main/LICENSE',
  upstream: 'https://github.com/aicodeking/pandacode',
} as const;

const link = (label: string, url: string) =>
  `<button type="button" class="secondary-button" data-action="open-url" data-url="${url}">${label}${icon('forward')}</button>`;

/** The About group: app version, license, the local-build update note, and links. */
export function aboutSettingsHtml(): string {
  return `<div class="settings-group settings-about" aria-labelledby="settings-about-heading">
    <div class="setting-row"><div><strong id="settings-about-heading">About PandaCode</strong><small class="settings-about-version">Version …</small></div></div>
    <div class="setting-row"><div><strong>License</strong><small>Open source (MIT-based). No account, no telemetry.</small></div>${link('PandaCode License', PROJECT_LINKS.license)}</div>
    <div class="setting-row settings-updates"><div><strong>Updates</strong><small class="settings-update-status">${LOCAL_BUILD_UPDATES}</small></div></div>
    <div class="setting-row"><div><strong>Source code</strong><small>The fork this build comes from.</small></div><div class="settings-row-actions">${link('GitHub repository', PROJECT_LINKS.repository)}${link('Issues', PROJECT_LINKS.issues)}</div></div>
    <div class="setting-row"><div><strong>Based on PandaCode</strong><small>The original open-source project.</small></div>${link('PandaCode', PROJECT_LINKS.upstream)}</div>
  </div>`;
}

/** Reads the bundle version from Tauri; the browser preview has no app bundle. */
async function appVersion(): Promise<string> {
  if (preview) return 'Preview';
  return (await import('@tauri-apps/api/app')).getVersion();
}

export function mountAboutSettings(host: HTMLElement): () => void {
  const version = host.querySelector<HTMLElement>('.settings-about-version');
  let disposed = false;
  if (version) {
    appVersion()
      .then(value => { if (!disposed) version.textContent = `Version ${value}`; })
      .catch(() => { if (!disposed) version.textContent = 'Version unavailable'; });
  }
  return () => { disposed = true; };
}
