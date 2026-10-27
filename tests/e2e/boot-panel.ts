import { installMocks } from "./mockBackend";

installMocks("panel");
await import("@/app/panel");
