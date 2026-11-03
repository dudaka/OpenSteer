import { DemoPage } from "./DemoPage";
import { Home } from "./Home";
import { useRoute } from "./route";

/** The catalog of demos at `#/`, one demo's page at `#/demo/<id>`. */
export function App() {
  const route = useRoute();
  return route.page === "demo" ? <DemoPage key={route.id} id={route.id} /> : <Home />;
}
