import { LOCAL_HOST, type RootState } from "@opensteer/core";
import { useSelector } from "react-redux";

/** Connection status and the demo list from the local steerd. */
export function App() {
  const host = useSelector((s: RootState) => s.hosts[LOCAL_HOST]);
  return (
    <main>
      <h1>OpenSteer</h1>
      <p data-status={host.status}>steerd: {host.status}</p>
      {host.demos.length === 0 ? (
        <p>No demos yet.</p>
      ) : (
        <ul>
          {host.demos.map((d) => (
            <li key={d.id}>{d.name}</li>
          ))}
        </ul>
      )}
    </main>
  );
}
