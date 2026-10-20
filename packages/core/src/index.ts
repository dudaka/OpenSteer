// @opensteer/core: transport, generated contract, Redux store.
import { hostActions, LOCAL_HOST, store } from "./store";
import { HostConnection, webSocketLink, type OpenLink } from "./transport";

export * from "./store";
export { HostConnection, webSocketLink, type ConnectionStatus, type Link, type OpenLink } from "./transport";
export type { DemoInfo } from "./contract/DemoInfo";

/** Typed calls against one host; results land in the store. */
export class Host {
  readonly conn: HostConnection;

  constructor(readonly id: string, link: OpenLink) {
    this.conn = new HostConnection(link, {
      onStatus: (status) => {
        store.dispatch(hostActions.status({ host: id, status }));
        if (status === "connected") void this.refresh();
      },
      onFrame: () => {},
    });
  }

  async refresh(): Promise<void> {
    const r = await this.conn.call({ method: "demo.list" });
    if (r.type === "demos") store.dispatch(hostActions.demos({ host: this.id, demos: r.value }));
  }
}

/** The host serving this page: same origin, /ws. */
export function localHost(): Host {
  const scheme = location.protocol === "https:" ? "wss" : "ws";
  return new Host(LOCAL_HOST, webSocketLink(`${scheme}://${location.host}/ws`));
}
