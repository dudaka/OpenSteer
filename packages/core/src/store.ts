// Redux store. Reducers only copy what the host sends: no derived state, no prediction.
// Everything is keyed by host id, as in caravan.
import { configureStore, createSlice, type PayloadAction } from "@reduxjs/toolkit";
import type { DemoInfo } from "./contract/DemoInfo";
import type { SceneFrame } from "./contract/SceneFrame";
import type { TerrainView } from "./contract/TerrainView";
import type { ConnectionStatus } from "./transport";

export const LOCAL_HOST = "local";

export interface HostState {
  status: ConnectionStatus;
  /** The demos the host offers, in sidebar order. */
  demos: DemoInfo[];
  /** The latest simulation frame, 2D (`flat`) or 3D (`space`); null until the first arrives. */
  frame: SceneFrame | null;
  /** Whether the host's simulation time is running (false when paused). */
  running: boolean;
  /** Id of the demo the host is showing; empty until the first frame. */
  demo: string;
  /** The latest ground the host sent and the demo it belongs to; null until the first arrives. */
  terrain: { demo: string; view: TerrainView | null } | null;
}

const emptyHost = (): HostState => ({ status: "connecting", demos: [], frame: null, running: false, demo: "", terrain: null });

const hosts = createSlice({
  name: "hosts",
  initialState: { [LOCAL_HOST]: emptyHost() } as Record<string, HostState>,
  reducers: {
    status(state, a: PayloadAction<{ host: string; status: ConnectionStatus }>) {
      state[a.payload.host].status = a.payload.status;
    },
    demos(state, a: PayloadAction<{ host: string; demos: DemoInfo[] }>) {
      state[a.payload.host].demos = a.payload.demos;
    },
    frame(state, a: PayloadAction<{ host: string; frame: SceneFrame; running: boolean; demo: string }>) {
      state[a.payload.host].frame = a.payload.frame;
      state[a.payload.host].running = a.payload.running;
      state[a.payload.host].demo = a.payload.demo;
    },
    terrain(state, a: PayloadAction<{ host: string; demo: string; view: TerrainView | null }>) {
      state[a.payload.host].terrain = { demo: a.payload.demo, view: a.payload.view };
    },
  },
});

export const hostActions = hosts.actions;

export const store = configureStore({ reducer: { hosts: hosts.reducer } });

export type RootState = ReturnType<typeof store.getState>;
