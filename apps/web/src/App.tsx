import { LOCAL_HOST, type RootState } from "@opensteer/core";
import { lazy, Suspense } from "react";
import { useSelector } from "react-redux";

// three.js is loaded in its own chunk, as caravan's MapView.
const Scene = lazy(() => import("./Scene").then((m) => ({ default: m.Scene })));

/** Connection status, the demo list and the scene from the local steerd. */
export function App() {
  // Select fields, not the host: a new frame replaces the host object 60 times a second.
  const status = useSelector((s: RootState) => s.hosts[LOCAL_HOST].status);
  const demos = useSelector((s: RootState) => s.hosts[LOCAL_HOST].demos);
  return (
    <main>
      <header>
        <h1>OpenSteer</h1>
        <p data-status={status}>steerd: {status}</p>
        {demos.length === 0 ? (
          <p>No demos yet.</p>
        ) : (
          <ul>
            {demos.map((d) => (
              <li key={d.id}>{d.name}</li>
            ))}
          </ul>
        )}
      </header>
      <Suspense>
        <Scene />
      </Suspense>
    </main>
  );
}
