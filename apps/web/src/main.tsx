import { localHost, store } from "@opensteer/core";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Provider } from "react-redux";
import { App } from "./App";
import { HostContext } from "./host";
import "./index.css";

const host = localHost();
host.conn.open();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Provider store={store}>
      <HostContext.Provider value={host}>
        <App />
      </HostContext.Provider>
    </Provider>
  </StrictMode>,
);
