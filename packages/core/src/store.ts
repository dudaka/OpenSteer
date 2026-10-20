// Redux store. Reducers only copy what the host sends: no derived state, no prediction.
// Everything is keyed by host id, as in caravan.
import { configureStore, createSlice, type PayloadAction } from "@reduxjs/toolkit";
import type { DemoInfo } from "./contract/DemoInfo";
import type { FrameView } from "./contract/FrameView";
import type { ConnectionStatus } from "./transport";

export const LOCAL_HOST = "local";

export interface HostState {
  status: ConnectionStatus;
  /** The demos the host offers, in sidebar order. */
  demos: DemoInfo[];
  /** The latest simulation frame; null until the first arrives. */
  frame: FrameView | null;
}

const emptyHost = (): HostState => ({ status: "connecting", demos: [], frame: null });

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
    frame(state, a: PayloadAction<{ host: string; frame: FrameView }>) {
      state[a.payload.host].frame = a.payload.frame;
    },
  },
});

export const hostActions = hosts.actions;

export const store = configureStore({ reducer: { hosts: hosts.reducer } });

export type RootState = ReturnType<typeof store.getState>;
