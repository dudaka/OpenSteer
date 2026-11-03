import { useEffect, useState } from "react";

/** The page in the URL hash: the catalog at `#/`, a demo at `#/demo/<id>`. */
export type Route = { page: "home" } | { page: "demo"; id: string };

export function parseRoute(hash: string): Route {
  const m = /^#\/demo\/([^/]+)$/.exec(hash);
  return m ? { page: "demo", id: decodeURIComponent(m[1]) } : { page: "home" };
}

export const HOME_HREF = "#/";
export const demoHref = (id: string) => `#/demo/${encodeURIComponent(id)}`;

/** The current route, following the browser's back and forward buttons. */
export function useRoute(): Route {
  const [route, setRoute] = useState(() => parseRoute(window.location.hash));
  useEffect(() => {
    const onChange = () => setRoute(parseRoute(window.location.hash));
    window.addEventListener("hashchange", onChange);
    return () => window.removeEventListener("hashchange", onChange);
  }, []);
  return route;
}
