import "../shared/styles.css";

import { createRoot } from "react-dom/client";
import { Hud } from "./Hud";

const root = document.getElementById("root");
if (!root) throw new Error("HUD root element missing");

createRoot(root).render(<Hud />);
