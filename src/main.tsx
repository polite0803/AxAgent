import React from "react";
import ReactDOM from "react-dom/client";
import AppRoot from "./App";
import "./index.css";

// Native context menu prevention is handled by GlobalCopyMenu component.
// It prevents the native menu while providing a custom Copy menu when text is selected.

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <AppRoot />
  </React.StrictMode>,
);
