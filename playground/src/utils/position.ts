/** Convert character offset to line/column (1-based for Monaco) */
export function offsetToLineColumn(
  source: string,
  offset: number,
): { line: number; column: number } {
  const beforeOffset = source.substring(0, offset);
  const lines = beforeOffset.split("\n");
  return {
    line: lines.length,
    column: lines[lines.length - 1].length + 1,
  };
}
