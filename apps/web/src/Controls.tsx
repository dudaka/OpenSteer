import { LOCAL_HOST, type RootState } from "@opensteer/core";
import { useSelector } from "react-redux";
import { useHost } from "./host";

/** Play/pause, single step and reset; the button label follows the host's `running`. */
export function Controls() {
  const host = useHost();
  const running = useSelector((s: RootState) => s.hosts[LOCAL_HOST].running);
  const connected = useSelector((s: RootState) => s.hosts[LOCAL_HOST].status === "connected");
  return (
    <div className="controls">
      <button disabled={!connected} onClick={() => void (running ? host.pause() : host.play())}>
        {running ? "Pause" : "Play"}
      </button>
      <button disabled={!connected} onClick={() => void host.step()}>
        Step
      </button>
      <button disabled={!connected} onClick={() => void host.reset()}>
        Reset
      </button>
    </div>
  );
}
