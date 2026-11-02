/* PandaCode's own updates. This is a local build of the fork: it has no updater and never
 * checks for, downloads or installs a release. A newer version is a rebuild from source.
 * The interface stays so the header pill and Settings → About keep one shape. */
export { describeUpdate, type UpdateMode, type UpdateState } from './app-updates-policy';
import type { UpdateMode, UpdateState } from './app-updates-policy';

export interface UpdatesHost {
  /** True while any agent is working. */
  busy(): boolean;
  notify(text: string): void;
}
export interface AppUpdates {
  readonly state: UpdateState;
  readonly mode: UpdateMode;
  readonly supported: boolean;
  readonly installable: boolean;
  subscribe(listener: () => void): () => void;
  setMode(mode: UpdateMode): void;
  check(): Promise<void>;
  restart(): Promise<void>;
}

/** What Settings → About says about updates in a local build. */
export const LOCAL_BUILD_UPDATES = 'Local build: it never checks for updates. Rebuild from source to update.';

let instance: AppUpdates | undefined;
export const appUpdates = (): AppUpdates | undefined => instance;

export function startAppUpdates(_host: UpdatesHost): AppUpdates {
  instance = {
    state: { kind: 'off' },
    mode: 'off',
    supported: false,
    installable: false,
    subscribe: () => () => {},
    setMode: () => {},
    check: async () => {},
    restart: async () => {},
  };
  return instance;
}

/** The header pill (`#update-status`) stays hidden: there is never an update to install. */
export function mountUpdatePill(pill: HTMLButtonElement, _updates: AppUpdates): void {
  pill.hidden = true;
}
