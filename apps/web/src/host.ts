import type { Host } from "@opensteer/core";
import { createContext, useContext } from "react";

export const HostContext = createContext<Host | null>(null);

export function useHost(): Host {
  const host = useContext(HostContext);
  if (!host) throw new Error("HostContext missing");
  return host;
}
