// Safe markdown rendering for derived §27 artifacts.
// Deliberately small: the derivation emits only this
// subset (headings, bold, lists, fenced code, plain
// paragraphs). Anything else renders as a plain text
// line. This file renders text only through React
// elements — it never injects raw HTML — so
// provider-supplied text inside a derived document
// can never become markup, a link, or a script. Link
// and image syntax is rendered as literal text.
import type { ReactNode } from 'react';

function renderInline(line: string, keyPrefix: string): ReactNode[] {
  const out: ReactNode[] = [];
  let rest = line;
  let key = 0;
  while (rest.length) {
    const codeAt = rest.indexOf('`');
    const boldAt = rest.indexOf('**');
    if (codeAt === -1 && boldAt === -1) {
      out.push(rest);
      break;
    }
    let at: number;
    let kind: 'code' | 'bold';
    if (codeAt !== -1 && (boldAt === -1 || codeAt < boldAt)) {
      at = codeAt;
      kind = 'code';
    } else {
      at = boldAt;
      kind = 'bold';
    }
    if (at > 0) {
      out.push(rest.slice(0, at));
      rest = rest.slice(at);
      continue;
    }
    if (kind === 'code') {
      const end = rest.indexOf('`', 1);
      if (end === -1) {
        out.push(rest);
        break;
      }
      out.push(<code key={`${keyPrefix}-${key++}`}>{rest.slice(1, end)}</code>);
      rest = rest.slice(end + 1);
    } else {
      const end = rest.indexOf('**', 2);
      if (end === -1) {
        out.push(rest);
        break;
      }
      out.push(<strong key={`${keyPrefix}-${key++}`}>{rest.slice(2, end)}</strong>);
      rest = rest.slice(end + 2);
    }
  }
  return out;
}

export function SafeMarkdown({ text }: { text: string }) {
  const blocks: ReactNode[] = [];
  const lines = text.split('\n');
  let i = 0;
  let key = 0;
  while (i < lines.length) {
    const line = lines[i];
    if (line.startsWith('```')) {
      const code: string[] = [];
      i += 1;
      while (i < lines.length && !lines[i].startsWith('```')) {
        code.push(lines[i]);
        i += 1;
      }
      i += 1; // closing fence (or end of text)
      blocks.push(
        <pre key={key++}>
          <code>{code.join('\n')}</code>
        </pre>,
      );
      continue;
    }
    const heading = line.match(/^(#{1,3}) (.*)$/);
    if (heading) {
      const level = heading[1].length;
      const content = renderInline(heading[2], `h${key}`);
      if (level === 1) blocks.push(<h4 key={key++}>{content}</h4>);
      else if (level === 2) blocks.push(<h5 key={key++}>{content}</h5>);
      else blocks.push(<h6 key={key++}>{content}</h6>);
      i += 1;
      continue;
    }
    if (line.startsWith('- ')) {
      const items: string[] = [];
      while (i < lines.length && lines[i].startsWith('- ')) {
        items.push(lines[i].slice(2));
        i += 1;
      }
      blocks.push(
        <ul key={key++}>
          {items.map((item, idx) => (
            <li key={idx}>{renderInline(item, `li${key}-${idx}`)}</li>
          ))}
        </ul>,
      );
      continue;
    }
    if (/^\d+\. /.test(line)) {
      const items: string[] = [];
      while (i < lines.length && /^\d+\. /.test(lines[i])) {
        items.push(lines[i].replace(/^\d+\. /, ''));
        i += 1;
      }
      blocks.push(
        <ol key={key++}>
          {items.map((item, idx) => (
            <li key={idx}>{renderInline(item, `ol${key}-${idx}`)}</li>
          ))}
        </ol>,
      );
      continue;
    }
    if (line.trim() === '') {
      i += 1;
      continue;
    }
    const paragraph: string[] = [];
    while (
      i < lines.length &&
      lines[i].trim() !== '' &&
      !lines[i].startsWith('```') &&
      !lines[i].startsWith('- ') &&
      !/^(#{1,3}) /.test(lines[i]) &&
      !/^\d+\. /.test(lines[i])
    ) {
      paragraph.push(lines[i]);
      i += 1;
    }
    blocks.push(<p key={key++}>{renderInline(paragraph.join(' '), `p${key}`)}</p>);
  }
  return <div>{blocks}</div>;
}
