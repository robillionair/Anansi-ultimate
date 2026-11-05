/**
 * Minimal safe Markdown subset renderer for imported documents
 * (M13 Slice 1). Headings, bold/italic, inline code, fenced code,
 * unordered lists and paragraphs — React elements only, never
 * `dangerouslySetInnerHTML`. Links are rendered as plain text
 * ("text (url)") with no anchor, and raw HTML is rendered as
 * literal text: nothing in an imported document can execute.
 */
import { createElement } from 'react';
import type { ReactNode } from 'react';

type Inline = { kind: 'text' | 'bold' | 'italic' | 'code'; text: string };

const INLINE = /(\*\*([^*]+)\*\*)|(\*([^*]+)\*)|(`([^`]+)`)|(\[([^\]]+)\]\(([^)]+)\))/g;

function parseInline(line: string): Inline[] {
  const parts: Inline[] = [];
  let at = 0;
  for (const match of line.matchAll(INLINE)) {
    const index = match.index ?? 0;
    if (index > at) parts.push({ kind: 'text', text: line.slice(at, index) });
    if (match[2] !== undefined) parts.push({ kind: 'bold', text: match[2] });
    else if (match[4] !== undefined) parts.push({ kind: 'italic', text: match[4] });
    else if (match[6] !== undefined) parts.push({ kind: 'code', text: match[6] });
    else if (match[8] !== undefined) parts.push({ kind: 'text', text: `${match[8]} (${match[9]})` });
    at = index + match[0].length;
  }
  if (at < line.length) parts.push({ kind: 'text', text: line.slice(at) });
  return parts;
}

function inlineNodes(parts: Inline[], keyPrefix: string): ReactNode[] {
  return parts.map((part, index) => {
    const key = `${keyPrefix}-${index}`;
    if (part.kind === 'bold') return createElement('strong', { key }, part.text);
    if (part.kind === 'italic') return createElement('em', { key }, part.text);
    if (part.kind === 'code') return createElement('code', { key }, part.text);
    return part.text;
  });
}

/** Render Markdown text as inert React elements. Empty text renders nothing. */
export function renderMarkdown(text: string): ReactNode[] {
  const lines = text.split('\n');
  const nodes: ReactNode[] = [];
  let index = 0;
  let key = 0;
  while (index < lines.length) {
    const line = lines[index];
    if (line.trim() === '') {
      index += 1;
      continue;
    }
    if (line.startsWith('```')) {
      const code: string[] = [];
      index += 1;
      while (index < lines.length && !lines[index].startsWith('```')) {
        code.push(lines[index]);
        index += 1;
      }
      index += 1;
      nodes.push(
        createElement('pre', { key: `md-${key++}` }, createElement('code', null, code.join('\n'))),
      );
      continue;
    }
    const heading = /^(#{1,3})\s+(.+)$/.exec(line);
    if (heading) {
      const level = heading[1].length;
      nodes.push(
        createElement(
          level === 1 ? 'h3' : level === 2 ? 'h4' : 'h5',
          { key: `md-${key++}` },
          ...inlineNodes(parseInline(heading[2]), `md-${key}`),
        ),
      );
      index += 1;
      continue;
    }
    if (/^[-*]\s+/.test(line)) {
      const items: string[] = [];
      while (index < lines.length && /^[-*]\s+/.test(lines[index])) {
        items.push(lines[index].replace(/^[-*]\s+/, ''));
        index += 1;
      }
      nodes.push(
        createElement(
          'ul',
          { key: `md-${key++}` },
          items.map((item, itemIndex) =>
            createElement(
              'li',
              { key: `md-li-${itemIndex}` },
              ...inlineNodes(parseInline(item), `md-li-${itemIndex}`),
            ),
          ),
        ),
      );
      continue;
    }
    const paragraph: string[] = [];
    while (
      index < lines.length &&
      lines[index].trim() !== '' &&
      !lines[index].startsWith('```') &&
      !/^(#{1,3})\s+/.test(lines[index]) &&
      !/^[-*]\s+/.test(lines[index])
    ) {
      paragraph.push(lines[index]);
      index += 1;
    }
    nodes.push(
      createElement(
        'p',
        { key: `md-${key++}` },
        ...inlineNodes(parseInline(paragraph.join(' ')), `md-${key}`),
      ),
    );
  }
  return nodes;
}
