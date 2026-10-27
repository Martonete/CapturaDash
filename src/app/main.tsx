import "@fontsource-variable/inter";
import "@/styles/tokens.css";
import "@/styles/base.css";
import "@/components/common/common.css";
import "@/components/clipboard/clipboard.css";
import "@/components/canvas/canvas.css";
import "./main.css";

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { initPersistence } from "@/features/canvas/persistence";
import { initBoards } from "@/stores/boardsStore";
import { initHistory } from "@/stores/historyStore";
import { initSettings } from "@/stores/settingsStore";

import { MainApp } from "./MainApp";

void initSettings();
void initHistory();
void initBoards();
initPersistence();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <MainApp />
  </StrictMode>,
);
