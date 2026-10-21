import { LOCAL_HOST, type RootState } from "@opensteer/core";
import { lazy, Suspense } from "react";
import { useSelector } from "react-redux";
import { Actions } from "./Actions";
import { Controls } from "./Controls";
import { Demos } from "./Demos";
import { Legend } from "./Legend";
import { Params } from "./Params";

// three.js is loaded in its own chunk, as caravan's MapView.
const Scene = lazy(() => import("./scene/Scene").then((m) => ({ default: m.Scene })));
const Scene3D = lazy(() => import("./scene/Scene3D").then((m) => ({ default: m.Scene3D })));

/** Connection status, the demo list, controls and the scene from the local steerd. */
export function App() {
  // Select fields, not the host: a new frame replaces the host object 60 times a second.
  const status = useSelector((s: RootState) => s.hosts[LOCAL_HOST].status);
  // A string, so the scene is swapped only when a demo of the other kind is selected.
  const kind = useSelector((s: RootState) => s.hosts[LOCAL_HOST].frame?.kind);
  return (
    <main>
      <header>
        <h1>OpenSteer</h1>
        <p data-status={status}>steerd: {status}</p>
        <Controls />
        <Actions />
        <Params />
      </header>
      <div className="body">
        <Demos />
        <div className="stage">
          <Suspense>
            {kind === "space" ? <Scene3D /> : <Scene />}
          </Suspense>
          <Legend />
        </div>
      </div>
    </main>
  );
}
