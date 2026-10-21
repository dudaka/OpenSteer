// Redux store. Reducers only copy what the host sends: no derived state, no prediction.
// Everything is keyed by host id, as in caravan.
import { configureStore, createSlice, type PayloadAction } from "@reduxjs/toolkit";
import type { DemoInfo } from "./contract/DemoInfo";
import type { SceneFrame } from "./contract/SceneFrame";
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
}

const emptyHost = (): HostState => ({ status: "connecting", demos: [], frame: null, running: false, demo: "" });

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
  },
});

export const hostActions = hosts.actions;

export const store = configureStore({ reducer: { hosts: hosts.reducer } });

export type RootState = ReturnType<typeof store.getState>;
