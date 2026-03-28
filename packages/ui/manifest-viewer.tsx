import type { ReactElement, ReactNode } from 'react';

export interface ManifestViewerProps {
  manifestMarkdown: string;
  manifestVersion: string;
  basisHash: string;
  generatedAt: string;
  sections?: string[];
  highlightBlockers?: boolean;
  onSectionClick?: (sectionId: string) => void;
}

export interface BoundedTextPreview {
  safeText: string;
  truncated: boolean;
  originalLength: number;
  maxChars: number;
  maxLines: number;
  originalLineCount: number;
}

const SPOOFING_CONTROL_PATTERN =
  /[\u200B-\u200F\u202A-\u202E\u2066-\u2069\uFEFF]/g;
const DEFAULT_MANIFEST_RENDER_LIMIT = 20_000;
const DEFAULT_MANIFEST_LINE_LIMIT = 400;

export function sanitizeUntrustedText(text: string): string {
  return text.replace(SPOOFING_CONTROL_PATTERN, (character) => {
    const codePoint = character.codePointAt(0);
    if (codePoint === undefined) {
      return '';
    }
    return `[U+${codePoint.toString(16).toUpperCase().padStart(4, '0')}]`;
  });
}

export function boundUntrustedText(
  text: string,
  maxChars = DEFAULT_MANIFEST_RENDER_LIMIT,
  maxLines = DEFAULT_MANIFEST_LINE_LIMIT,
): BoundedTextPreview {
  const safeText = sanitizeUntrustedText(text);
  const lines = safeText.split('\n');
  const boundedByLines =
    lines.length > maxLines ? lines.slice(0, maxLines).join('\n') : safeText;

  if (boundedByLines.length <= maxChars && lines.length <= maxLines) {
    return {
      safeText: boundedByLines,
      truncated: false,
      originalLength: safeText.length,
      maxChars,
      maxLines,
      originalLineCount: lines.length,
    };
  }

  return {
    safeText: boundedByLines.slice(0, maxChars),
    truncated: true,
    originalLength: safeText.length,
    maxChars,
    maxLines,
    originalLineCount: lines.length,
  };
}

function slugify(value: string): string {
  return sanitizeUntrustedText(value)
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '');
}

function isSafeUrl(url: string): boolean {
  const normalized = sanitizeUntrustedText(url.trim());
  if (normalized.startsWith('/') || normalized.startsWith('#') || normalized.startsWith('?')) {
    return true;
  }

  try {
    const parsed = new URL(normalized);
    return parsed.protocol === 'http:' || parsed.protocol === 'https:' || parsed.protocol === 'mailto:';
  } catch {
    return false;
  }
}

function renderInline(text: string): ReactNode[] {
  const nodes: ReactNode[] = [];
  const linkPattern = /\[([^\]]+)\]\(([^)]+)\)/g;
  let cursor = 0;

  for (const match of text.matchAll(linkPattern)) {
    const fullMatch = match[0];
    const label = sanitizeUntrustedText(match[1] ?? '');
    const target = sanitizeUntrustedText(match[2] ?? '');
    const start = match.index ?? 0;

    if (start > cursor) {
      nodes.push(
        <span key={`text-${cursor}`}>{sanitizeUntrustedText(text.slice(cursor, start))}</span>,
      );
    }

    if (isSafeUrl(target)) {
      nodes.push(
        <a href={target} key={`link-${start}`} rel="noreferrer" target="_blank">
          {label}
        </a>,
      );
    } else {
      nodes.push(<span key={`label-${start}`}>{label}</span>);
    }

    cursor = start + fullMatch.length;
  }

  if (cursor < text.length) {
    nodes.push(<span key={`tail-${cursor}`}>{sanitizeUntrustedText(text.slice(cursor))}</span>);
  }

  return nodes;
}

function extractSections(markdown: string): string[] {
  return markdown
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => line.startsWith('#'))
    .map((line) => line.replace(/^#+\s*/, ''))
    .filter(Boolean);
}

function renderMarkdown(markdown: string, highlightBlockers: boolean): ReactNode[] {
  const lines = markdown.split('\n');
  const nodes: ReactNode[] = [];
  let listItems: ReactNode[] = [];

  const flushList = (): void => {
    if (listItems.length > 0) {
      nodes.push(<ul key={`list-${nodes.length}`}>{listItems}</ul>);
      listItems = [];
    }
  };

  for (const line of lines) {
    const trimmed = line.trim();
    if (trimmed === '') {
      flushList();
      continue;
    }

    if (trimmed.startsWith('#')) {
      flushList();
      const title = trimmed.replace(/^#+\s*/, '');
      const sectionId = slugify(title);
      const blocker = highlightBlockers && /blocker/i.test(title);
      nodes.push(
        <section data-section={sectionId} key={`section-${sectionId}`}>
          <h2 id={sectionId}>{title}</h2>
          {blocker ? <span data-blocker="true">Blocker</span> : null}
          <button type="button" data-copy-id={sectionId}>
            Copy section id
          </button>
        </section>,
      );
      continue;
    }

    if (trimmed.startsWith('- ')) {
      listItems.push(<li key={`item-${listItems.length}`}>{renderInline(trimmed.slice(2))}</li>);
      continue;
    }

    flushList();
    nodes.push(<p key={`paragraph-${nodes.length}`}>{renderInline(trimmed)}</p>);
  }

  flushList();
  return nodes;
}

export function ManifestViewer({
  manifestMarkdown,
  manifestVersion,
  basisHash,
  generatedAt,
  sections,
  highlightBlockers = false,
  onSectionClick,
}: ManifestViewerProps): ReactElement {
  const boundedManifest = boundUntrustedText(manifestMarkdown);
  const sectionHeadings = (sections ?? extractSections(boundedManifest.safeText)).map((section) =>
    sanitizeUntrustedText(section),
  );

  return (
    <article className="manifest-viewer">
      <header>
        <p>Manifest version {sanitizeUntrustedText(manifestVersion)}</p>
        <p>Basis hash {sanitizeUntrustedText(basisHash)}</p>
        <p>Generated at {sanitizeUntrustedText(generatedAt)}</p>
      </header>

      <nav aria-label="Manifest sections">
        <ul>
          {sectionHeadings.map((section) => {
            const sectionId = slugify(section);
            return (
              <li key={sectionId}>
                <button type="button" onClick={() => onSectionClick?.(sectionId)}>
                  {section}
                </button>
              </li>
            );
          })}
        </ul>
      </nav>

      <div className="manifest-viewer__content">
        {renderMarkdown(boundedManifest.safeText, highlightBlockers)}
      </div>
      {boundedManifest.truncated ? (
        <p className="manifest-viewer__status" role="status">
          Manifest truncated to the first {boundedManifest.maxLines.toLocaleString('en-GB')}{' '}
          lines / {boundedManifest.maxChars.toLocaleString('en-GB')} characters of{' '}
          {boundedManifest.originalLineCount.toLocaleString('en-GB')} lines and{' '}
          {boundedManifest.originalLength.toLocaleString('en-GB')} characters. Open the archived
          artifact for the full payload.
        </p>
      ) : null}
    </article>
  );
}
