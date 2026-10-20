// @opensteer/core: transport, generated contract, Redux store.
import { hostActions, LOCAL_HOST, store } from "./store";
import { HostConnection, webSocketLink, type OpenLink } from "./transport";

export * from "./store";
export { HostConnection, webSocketLink, type ConnectionStatus, type Link, type OpenLink } from "./transport";
export type { ActionView } from "./contract/ActionView";
export type { DemoInfo } from "./contract/DemoInfo";
export type { FrameView } from "./contract/FrameView";
export type { LabelView } from "./contract/LabelView";
export type { LineView } from "./contract/LineView";
export type { TrailView } from "./contract/TrailView";
export type { VehicleView } from "./contract/VehicleView";

/** Typed calls against one host; results land in the store. */
export class Host {
  readonly conn: HostConnection;

  constructor(readonly id: string, link: OpenLink) {
    this.conn = new HostConnection(link, {
      onStatus: (status) => {
        store.dispatch(hostActions.status({ host: id, status }));
        if (status === "connected") void this.refresh();
      },
      onFrame: (frame) => {
        if (frame.t === "frame") {
          store.dispatch(hostActions.frame({ host: id, frame: frame.frame, running: frame.running, demo: frame.demo }));
        }
      },
    });
  }

  async refresh(): Promise<void> {
    const r = await this.conn.call({ method: "demo.list" });
    if (r.type === "demos") store.dispatch(hostActions.demos({ host: this.id, demos: r.value }));
  }

  // Demo and sim calls: the effect arrives in the next frame, which carries `demo` and `running`.
  async select(demo: string): Promise<void> {
    await this.conn.call({ method: "demo.select", params: { id: demo } });
  }

  /** A click on the ground at world point (x, y). */
  async click(x: number, y: number): Promise<void> {
    await this.conn.call({ method: "demo.click", params: { x, y } });
  }

  async action(id: string): Promise<void> {
    await this.conn.call({ method: "demo.action", params: { id } });
  }

  async play(): Promise<void> {
    await this.conn.call({ method: "sim.play" });
  }

  async pause(): Promise<void> {
    await this.conn.call({ method: "sim.pause" });
  }

  async step(): Promise<void> {
    await this.conn.call({ method: "sim.step" });
  }

  async reset(): Promise<void> {
    await this.conn.call({ method: "sim.reset" });
  }
}

/** The host serving this page: same origin, /ws. */
export function localHost(): Host {
  const scheme = location.protocol === "https:" ? "wss" : "ws";
  return new Host(LOCAL_HOST, webSocketLink(`${scheme}://${location.host}/ws`));
}
