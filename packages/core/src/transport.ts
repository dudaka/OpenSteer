// One link to one steerd host, MessagePack frames over a WebSocket. Requests are matched to
// replies by id; other frames go to the frame listener. Reconnects with backoff.
import { decode, encode } from "@msgpack/msgpack";
import type { Call } from "./contract/Call";
import type { HostFrame } from "./contract/HostFrame";
import type { Reply } from "./contract/Reply";
import type { Request } from "./contract/Request";

export type ConnectionStatus = "connecting" | "connected" | "disconnected";

export interface HostConnectionEvents {
  onStatus(status: ConnectionStatus): void;
  onFrame(frame: HostFrame): void;
}

/** What carries a host's frames, whole MessagePack frames in and out. */
export interface Link {
  send(bytes: Uint8Array<ArrayBuffer>): void;
  close(): void;
}

export interface LinkEvents {
  onOpen(): void;
  onMessage(bytes: Uint8Array): void;
  onClose(): void;
}

export type OpenLink = (events: LinkEvents) => Link;

export function webSocketLink(url: string): OpenLink {
  return (events) => {
    const socket = new WebSocket(url);
    socket.binaryType = "arraybuffer";
    socket.onopen = () => events.onOpen();
    socket.onmessage = (msg) => events.onMessage(new Uint8Array(msg.data as ArrayBuffer));
    socket.onclose = () => events.onClose();
    return { send: (bytes) => socket.send(bytes), close: () => socket.close() };
  };
}

export class HostConnection {
  private link: Link | null = null;
  private ready = false;
  private nextId = 1;
  private pending = new Map<number, { resolve(r: Reply): void; reject(e: Error): void }>();
  private retryMs = 500;
  private closed = false;

  constructor(
    private readonly openLink: OpenLink,
    private readonly events: HostConnectionEvents,
  ) {}

  open(): void {
    this.closed = false;
    this.events.onStatus("connecting");
    this.link = this.openLink({
      onOpen: () => {
        this.ready = true;
        this.retryMs = 500;
        this.events.onStatus("connected");
      },
      onMessage: (bytes) => this.receive(decode(bytes) as HostFrame),
      onClose: () => {
        this.link = null;
        this.ready = false;
        this.events.onStatus("disconnected");
        for (const p of this.pending.values()) p.reject(new Error("connection to the host closed"));
        this.pending.clear();
        if (!this.closed) {
          setTimeout(() => this.open(), this.retryMs);
          this.retryMs = Math.min(this.retryMs * 2, 10_000);
        }
      },
    });
  }

  close(): void {
    this.closed = true;
    this.link?.close();
  }

  /** Send one call; resolves with the reply or rejects with the host's error. */
  call(call: Call): Promise<Reply> {
    const link = this.link;
    if (!link || !this.ready) {
      return Promise.reject(new Error("not connected to the host"));
    }
    const request: Request = { id: this.nextId++, call };
    return new Promise((resolve, reject) => {
      this.pending.set(request.id, { resolve, reject });
      link.send(encode(request));
    });
  }

  private receive(frame: HostFrame): void {
    if (frame.t === "res") {
      const p = this.pending.get(frame.id);
      if (!p) return;
      this.pending.delete(frame.id);
      if (frame.result) p.resolve(frame.result);
      else p.reject(new Error(frame.error ?? "unknown error"));
      return;
    }
    this.events.onFrame(frame);
  }
}
