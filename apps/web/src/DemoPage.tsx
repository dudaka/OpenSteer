import { LOCAL_HOST, type RootState } from "@opensteer/core";
import { lazy, Suspense, useEffect } from "react";
import { useSelector } from "react-redux";
import { Actions } from "./Actions";
import { Controls } from "./Controls";
import { useHost } from "./host";
import { Legend } from "./Legend";
import { Params } from "./Params";
import { demoHref, HOME_HREF } from "./route";

// three.js is loaded in its own chunk, as caravan's MapView.
const Scene = lazy(() => import("./scene/Scene").then((m) => ({ default: m.Scene })));
const Scene3D = lazy(() => import("./scene/Scene3D").then((m) => ({ default: m.Scene3D })));

/**
 * One demo's page: the controls on the left (play/pause/step/reset, the demo's buttons and
 * sliders, the way back to the catalog and to the neighbouring demos), the scene on the right.
 * Opening the page selects the demo on the host once connected.
 */
export function DemoPage({ id }: { id: string }) {
  const host = useHost();
  // Select fields, not the host: a new frame replaces the host object 60 times a second.
  const status = useSelector((s: RootState) => s.hosts[LOCAL_HOST].status);
  const demos = useSelector((s: RootState) => s.hosts[LOCAL_HOST].demos);
  const current = useSelector((s: RootState) => s.hosts[LOCAL_HOST].demo);
  // A string, so the scene is swapped only when a demo of the other kind is shown.
  const kind = useSelector((s: RootState) => s.hosts[LOCAL_HOST].frame?.kind);
  const index = demos.findIndex((d) => d.id === id);
  const known = index >= 0;
  useEffect(() => {
    if (status === "connected" && known && current !== id) void host.select(id);
  }, [status, known, current, id, host]);
  useEffect(() => {
    document.title = known ? `${demos[index].name} - OpenSteer` : "OpenSteer";
  }, [known, demos, index]);
  if (demos.length > 0 && !known) {
    return (
      <main>
        <p>
          No demo called {id}. <a href={HOME_HREF}>All demos</a>
        </p>
      </main>
    );
  }
  const neighbour = (step: number) => demos[index + step];
  const [previous, next] = [neighbour(-1), neighbour(1)];
  return (
    <main className="demo-page">
      <aside className="panel">
        <a className="back" href={HOME_HREF}>
          All demos
        </a>
        <h1>{known ? demos[index].name : id}</h1>
        <p data-status={status}>steerd: {status}</p>
        <Controls />
        <Actions />
        <Params />
        <nav className="neighbours">
          {previous && <a href={demoHref(previous.id)}>Previous: {previous.name}</a>}
          {next && <a href={demoHref(next.id)}>Next: {next.name}</a>}
        </nav>
      </aside>
      <div className="stage">
        {/* Frames of the demo shown before the switch are not drawn under this demo's name. */}
        {current === id ? (
          <Suspense>{kind === "space" ? <Scene3D /> : <Scene />}</Suspense>
        ) : (
          <p className="waiting">Opening {known ? demos[index].name : id}...</p>
        )}
        <Legend />
      </div>
    </main>
  );
}
