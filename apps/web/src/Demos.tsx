import { LOCAL_HOST, type RootState } from "@opensteer/core";
import { useSelector } from "react-redux";
import { useHost } from "./host";

/** The host's demos; clicking one selects it, the host's current demo is highlighted. */
export function Demos() {
  const host = useHost();
  const demos = useSelector((s: RootState) => s.hosts[LOCAL_HOST].demos);
  const current = useSelector((s: RootState) => s.hosts[LOCAL_HOST].demo);
  if (demos.length === 0) return <p>No demos yet.</p>;
  return (
    <nav className="demos">
      {demos.map((d) => (
        <button key={d.id} aria-pressed={d.id === current} onClick={() => void host.select(d.id)}>
          {d.name}
        </button>
      ))}
    </nav>
  );
}
