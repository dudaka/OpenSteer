import { LOCAL_HOST, type ParamView, type RootState } from "@opensteer/core";
import { useSelector } from "react-redux";
import { useHost } from "./host";

const NONE: ParamView[] = [];

/** Same sliders, same values: frames arrive 60 times a second, sliders rarely change. */
const sameParams = (a: ParamView[], b: ParamView[]) =>
  a.length === b.length && a.every((x, i) => x.id === b[i].id && x.value === b[i].value);

/** The current demo's sliders; the value shown is the one the host sent back. */
export function Params() {
  const host = useHost();
  const params = useSelector((s: RootState) => s.hosts[LOCAL_HOST].frame?.params ?? NONE, sameParams);
  if (params.length === 0) return null;
  return (
    <div className="controls">
      {params.map((p) => (
        <label key={p.id} className="param">
          {p.label} {p.value.toFixed(2)}
          <input
            type="range"
            min={p.min}
            max={p.max}
            step={p.step}
            value={p.value}
            onChange={(e) => void host.param(p.id, Number(e.target.value))}
          />
        </label>
      ))}
    </div>
  );
}
