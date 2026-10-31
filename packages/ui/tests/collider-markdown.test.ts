// M8 §27 safe markdown renderer, node side: real
// server rendering (react-dom/server) proves headings,
// lists, bold, and code render as elements — and that
// hostile text never becomes markup.
import test from 'node:test';
import assert from 'node:assert/strict';
import { renderToStaticMarkup } from 'react-dom/server';
import { createElement } from 'react';
import { SafeMarkdown } from '../src/colliderMarkdown';

test('renderer covers the derived-document subset', () => {
  const html = renderToStaticMarkup(
    createElement(SafeMarkdown, {
      text: [
        '# Title',
        '',
        '## Section',
        '',
        'A **bold** line with `code`.',
        '',
        '- first item',
        '- second item',
        '',
        '1. one',
        '2. two',
        '',
        '```',
        'plain code <b>not markup</b>',
        '```',
      ].join('\n'),
    }),
  );
  assert.ok(html.includes('<h4>Title</h4>'), html);
  assert.ok(html.includes('<h5>Section</h5>'), html);
  assert.ok(html.includes('<strong>bold</strong>'), html);
  assert.ok(html.includes('<code>code</code>'), html);
  assert.ok(html.includes('<ul>') && html.includes('<li>first item</li>'), html);
  assert.ok(html.includes('<ol>') && html.includes('<li>one</li>'), html);
  // Code-fence contents are escaped, never markup.
  assert.ok(html.includes('&lt;b&gt;not markup&lt;/b&gt;'), html);
  assert.ok(!html.includes('<b>not markup</b>'), html);
});

test('hostile text renders as escaped text, never markup', () => {
  const html = renderToStaticMarkup(
    createElement(SafeMarkdown, {
      text: [
        '# Heading',
        '',
        '<script>alert(1)</script>',
        '',
        '[click me](https://example.invalid) and ![img](https://example.invalid/x.png)',
        '',
        'Text with **unclosed bold and `unclosed code.',
      ].join('\n'),
    }),
  );
  assert.ok(!html.includes('<script>'), html);
  assert.ok(html.includes('&lt;script&gt;'), html);
  assert.ok(!html.includes('<a href'), html);
  assert.ok(!html.includes('<img'), html);
  assert.ok(html.includes('unclosed bold'), html);
});

test('a document over 1 MB renders without failure', () => {
  const line = 'A long derived line that repeats to grow the document. ';
  const text = `# Big\n\n${line.repeat(22000)}\n\n- ${'item '.repeat(20000)}\n`;
  assert.ok(text.length > 1_000_000, `fixture is ${text.length} chars`);
  const html = renderToStaticMarkup(createElement(SafeMarkdown, { text }));
  assert.ok(html.includes('<h4>Big</h4>'), 'heading survives');
  assert.ok(html.length > 1_000_000, 'output carries the content');
});
