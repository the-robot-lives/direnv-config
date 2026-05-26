type Segment =
  | { type: 'key'; value: string }
  | { type: 'index'; value: number }
  | { type: 'wildcard' }
  | { type: 'length' };

export function parsePath(path: string): Segment[] {
  const segments: Segment[] = [];
  if (!path) return segments;

  for (const token of path.split('.')) {
    if (token === 'length' && segments.length > 0) {
      segments.push({ type: 'length' });
      continue;
    }

    const bracketPos = token.indexOf('[');
    if (bracketPos !== -1) {
      const keyPart = token.slice(0, bracketPos);
      if (keyPart) {
        segments.push({ type: 'key', value: keyPart });
      }

      let rest = token.slice(bracketPos);
      while (rest.includes('[')) {
        const open = rest.indexOf('[');
        const close = rest.indexOf(']');
        const inner = rest.slice(open + 1, close);
        if (inner === '*') {
          segments.push({ type: 'wildcard' });
        } else {
          segments.push({ type: 'index', value: parseInt(inner, 10) });
        }
        rest = rest.slice(close + 1);
      }
    } else {
      segments.push({ type: 'key', value: token });
    }
  }

  return segments;
}

function resolveIndex(idx: number, len: number): number | null {
  const resolved = idx < 0 ? len + idx : idx;
  if (resolved < 0 || resolved >= len) return null;
  return resolved;
}

function getSegments(current: unknown, segments: Segment[]): unknown {
  if (segments.length === 0) return current;

  const [seg, ...rest] = segments;

  switch (seg.type) {
    case 'key': {
      if (current === null || current === undefined || typeof current !== 'object' || Array.isArray(current)) {
        return null;
      }
      const map = current as Record<string, unknown>;
      if (!(seg.value in map)) return null;
      return getSegments(map[seg.value], rest);
    }
    case 'index': {
      if (!Array.isArray(current)) return null;
      const resolved = resolveIndex(seg.value, current.length);
      if (resolved === null) return null;
      return getSegments(current[resolved], rest);
    }
    case 'wildcard': {
      if (!Array.isArray(current)) return null;
      return current
        .map((elem) => getSegments(elem, rest))
        .filter((v) => v !== null);
    }
    case 'length': {
      if (rest.length > 0) return null;
      if (Array.isArray(current)) return current.length;
      if (current !== null && typeof current === 'object') return Object.keys(current as object).length;
      return null;
    }
  }
}

export function getPath(root: unknown, path: string): unknown {
  const segments = parsePath(path);
  return getSegments(root, segments);
}
