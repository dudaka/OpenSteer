import { LOCAL_HOST, type ActionView, type RootState } from "@opensteer/core";
import { useSelector } from "react-redux";
import { useHost } from "./host";

const NONE: ActionView[] = [];

/** Same buttons, same pressed state: frames arrive 60 times a second, buttons rarely change. */
const sameActions = (a: ActionView[], b: ActionView[]) =>
  a.length === b.length && a.every((x, i) => x.id === b[i].id && x.label === b[i].label && x.active === b[i].active);

/** The current demo's own buttons, as the host lists them in each frame. */
export function Actions() {
  const host = useHost();
  const actions = useSelector((s: RootState) => s.hosts[LOCAL_HOST].frame?.actions ?? NONE, sameActions);
  if (actions.length === 0) return null;
  return (
    <div className="controls">
      {actions.map((a) => (
        <button key={a.id} aria-pressed={a.active} onClick={() => void host.action(a.id)}>
          {a.label}
        </button>
      ))}
    </div>
  );
}
