import "@fontsource-variable/inter";
import "@/styles/tokens.css";
import "@/styles/base.css";
import "./toast.css";

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { initSettings } from "@/stores/settingsStore";

import { ToastApp } from "./ToastApp";

void initSettings();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ToastApp />
  </StrictMode>,
);
