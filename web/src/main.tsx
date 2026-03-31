/** @jsxImportSource @b9g/crank */

import "./style.css";
import { renderer } from "@b9g/crank/dom";
import { Page } from "./Page.js";

(async () => {
  await renderer.render(<Page />, document.body);
})();
