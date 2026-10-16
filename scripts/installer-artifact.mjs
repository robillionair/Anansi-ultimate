import { createHash } from 'node:crypto';
export function nsisPayloadHash(binary) {
  const payload=Buffer.from(binary);
  const marker=Buffer.from('__TAURI_BUNDLE_TYPE_VAR_UNK');
  const offset=payload.indexOf(marker);
  if(offset<0||payload.indexOf(marker,offset+1)!==-1)throw Error('Unexpected bundle marker; inspect toolchain before packaging.');
  Buffer.from('__TAURI_BUNDLE_TYPE_VAR_NSS').copy(payload,offset);
  return createHash('sha256').update(payload).digest('hex');
}
