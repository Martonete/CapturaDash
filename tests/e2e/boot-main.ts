import { installMocks } from "./mockBackend";

installMocks("main");
await import("@/app/main");
