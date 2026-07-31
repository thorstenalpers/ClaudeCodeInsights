/**
 * The single channel between the UI and the host. Nothing else in the app may
 * talk to `chrome.webview` directly.
 */

export type BridgeErrorPayload = { message: string; code?: string };

type BridgeResponse =
  | { id: string; ok: true; result: unknown }
  | { id: string; ok: false; error: BridgeErrorPayload };

type BridgeEvent = { event: string; payload: unknown };

type WebViewApi = {
  postMessage(message: unknown): void;
  addEventListener(type: 'message', listener: (event: { data: unknown }) => void): void;
};

declare global {
  interface Window {
    chrome?: { webview?: WebViewApi };
  }
}

export class BridgeCallError extends Error {
  constructor(
    message: string,
    readonly code?: string,
  ) {
    super(message);
    this.name = 'BridgeCallError';
  }
}

const CALL_TIMEOUT_MS = 30_000;

type Pending = {
  resolve: (value: unknown) => void;
  reject: (reason: unknown) => void;
  timer: ReturnType<typeof setTimeout>;
};

class Bridge {
  #pending = new Map<string, Pending>();
  #listeners = new Map<string, Set<(payload: unknown) => void>>();
  #nextId = 0;
  #webview: WebViewApi | undefined;

  constructor() {
    this.#webview = window.chrome?.webview;
    this.#webview?.addEventListener('message', (event) => this.#receive(event.data));
  }

  /** False when running under `npm run dev` or Storybook, with no host attached. */
  get isHosted(): boolean {
    return this.#webview !== undefined;
  }

  async call<T>(method: string, params?: unknown): Promise<T> {
    if (!this.#webview) {
      throw new BridgeCallError(`No host attached; '${method}' cannot be called.`, 'no-host');
    }

    const id = String(++this.#nextId);
    const promise = new Promise<unknown>((resolve, reject) => {
      const timer = setTimeout(() => {
        this.#pending.delete(id);
        reject(new BridgeCallError(`'${method}' timed out.`, 'timeout'));
      }, CALL_TIMEOUT_MS);
      this.#pending.set(id, { resolve, reject, timer });
    });

    this.#webview.postMessage({ id, method, params: params ?? null });
    return (await promise) as T;
  }

  on(event: string, listener: (payload: unknown) => void): () => void {
    let set = this.#listeners.get(event);
    if (!set) {
      set = new Set();
      this.#listeners.set(event, set);
    }
    set.add(listener);
    return () => set.delete(listener);
  }

  #receive(data: unknown): void {
    if (typeof data !== 'object' || data === null) return;

    if ('event' in data) {
      const { event, payload } = data as BridgeEvent;
      this.#listeners.get(event)?.forEach((listener) => listener(payload));
      return;
    }

    const message = data as BridgeResponse;
    const pending = this.#pending.get(message.id);
    if (!pending) return;

    clearTimeout(pending.timer);
    this.#pending.delete(message.id);

    if (message.ok) {
      pending.resolve(message.result);
    } else {
      pending.reject(new BridgeCallError(message.error.message, message.error.code));
    }
  }
}

export const bridge = new Bridge();
