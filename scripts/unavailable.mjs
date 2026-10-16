const operation = process.argv[2];
console.error(operation === 'backup'
  ? 'Source backup is not implemented yet; M0.3 owns independent backup/restore. No backup was created.'
  : 'Product build is not available until M1. Run pnpm check and pnpm test for foundation verification.');
process.exitCode = 1;
