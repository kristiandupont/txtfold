/** @jsxImportSource @b9g/crank */

import "./markdown.css";
import { marked } from "marked";

export function Markdown({ content }: { content: string }) {
  const html = marked(content, { async: false }) as string;
  return <div class="markdown-body" innerHTML={html} />;
}
