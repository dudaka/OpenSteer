import { LOCAL_HOST, type DemoInfo, type RootState } from "@opensteer/core";
import { useEffect } from "react";
import { useSelector } from "react-redux";
import { demoHref } from "./route";

const is3d = (d: DemoInfo) => d.id.startsWith("3d-");

/** The main page: every demo the host offers as a card, 2D then 3D; a card opens its page. */
export function Home() {
  const status = useSelector((s: RootState) => s.hosts[LOCAL_HOST].status);
  const demos = useSelector((s: RootState) => s.hosts[LOCAL_HOST].demos);
  const current = useSelector((s: RootState) => s.hosts[LOCAL_HOST].demo);
  useEffect(() => {
    document.title = "OpenSteer";
  }, []);
  const group = (title: string, list: DemoInfo[]) =>
    list.length > 0 && (
      <section>
        <h2>{title}</h2>
        <div className="cards">
          {list.map((d, i) => (
            <a key={d.id} className="card" href={demoHref(d.id)} aria-current={d.id === current ? "true" : undefined}>
              <span className="card-index">{i + 1}</span>
              {d.name}
            </a>
          ))}
        </div>
      </section>
    );
  return (
    <main className="home">
      <header>
        <h1>OpenSteer</h1>
        <p data-status={status}>steerd: {status}</p>
      </header>
      <p className="lead">
        Craig Reynolds' steering behaviors, ported to Rust and drawn here. Pick a demo; its page has the controls
        on the left.
      </p>
      {demos.length === 0 ? <p>No demos yet.</p> : null}
      {group("2D, on the ground plane", demos.filter((d) => !is3d(d)))}
      {group("3D", demos.filter(is3d))}
    </main>
  );
}
