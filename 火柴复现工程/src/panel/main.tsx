import "../shared/styles.css";

import { createRoot } from "react-dom/client";
import { Panel } from "./Panel";

const root = document.getElementById("root");
if (!root) throw new Error("Panel root element missing");

createRoot(root).render(<Panel />);
