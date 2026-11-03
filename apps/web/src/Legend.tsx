import { LOCAL_HOST, type LegendView, type RootState } from "@opensteer/core";
import { useState } from "react";
import { useSelector } from "react-redux";

const NONE: LegendView[] = [];

/** Same entries: frames arrive 60 times a second, the legend only changes with the demo. */
const sameLegend = (a: LegendView[], b: LegendView[]) =>
  a.length === b.length && a.every((x, i) => x.color === b[i].color && x.label === b[i].label);

const hex = (color: number) => `#${color.toString(16).padStart(6, "0")}`;

/**
 * What each line color means in the current demo, as the host lists it in each frame, behind
 * a button: folded by default so it does not cover the scene.
 */
export function Legend() {
  const legend = useSelector((s: RootState) => s.hosts[LOCAL_HOST].frame?.legend ?? NONE, sameLegend);
  const [open, setOpen] = useState(false);
  if (legend.length === 0) return null;
  return (
    <div className="legend-box">
      <button type="button" className="legend-toggle" onClick={() => setOpen(!open)} aria-expanded={open}>
        {open ? "Legend \u25BE" : `Legend (${legend.length}) \u25B8`}
      </button>
      {open && (
        <ul className="legend" aria-label="Line colors">
          {legend.map((l) => (
            <li key={l.color}>
              <i style={{ background: hex(l.color) }} />
              {l.label}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
