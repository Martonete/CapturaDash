import "@fontsource-variable/inter";
import "@/styles/tokens.css";
import "@/styles/base.css";
import "@/components/common/common.css";
import "@/components/clipboard/clipboard.css";
import "@/components/sidebar/panel.css";

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { PanelApp } from "@/components/sidebar/PanelApp";
import { initBoards } from "@/stores/boardsStore";
import { initHistory } from "@/stores/historyStore";
import { initSettings } from "@/stores/settingsStore";

void initSettings();
void initHistory();
void initBoards();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <PanelApp />
  </StrictMode>,
);
