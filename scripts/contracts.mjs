import { cargo } from './cargo.mjs';
import { compile } from 'json-schema-to-typescript';
import fs from 'node:fs';
import Ajv2020 from 'ajv/dist/2020.js';
import standaloneCode from 'ajv/dist/standalone/index.js';
const result=cargo(['run','--quiet','--locked','-p','anansi-contracts']);
if(result.status!==0)throw Error(result.stderr||'Contract generator failed');
const schemas=JSON.parse(result.stdout);
let types='// Generated from Rust serde DTOs. Run pnpm contracts:generate.\n';
for(const [name,schema] of Object.entries(schemas))types+=await compile(schema,name,{bannerComment:'',additionalProperties:false});
// Independently compiled DTOs can share named enum definitions. Keep one identical
// declaration and reject collisions rather than emitting duplicate type aliases.
const aliases=new Map();
types=types.replace(/^export type (\w+) =[\s\S]*?;\n(?=\n*(?:export |$))/gm,(declaration,name)=>{
  const previous=aliases.get(name);
  if(previous&&previous!==declaration)throw Error('Conflicting generated alias: '+name);
  aliases.set(name,declaration);return previous?'':declaration;
});
const ajv=new Ajv2020({strict:false,formats:{uint8:true,uint32:true,uint64:true,int64:true,double:true},code:{source:true,esm:true}});
for(const [name,schema] of Object.entries(schemas))ajv.addSchema(schema,name);
const names=Object.keys(schemas);
const validatorName=name=>'valid'+(name==='CommandError'?'Error':name);
const validators=standaloneCode(ajv,Object.fromEntries(names.map(name=>[validatorName(name),name])))+'\n';
const declarations=`// Generated from Rust schemas.\nimport type {${names.join(',')}} from './generated';\n`+names.map(name=>`export function ${validatorName(name)}(value:unknown):value is ${name};\n`).join('');
for(const [file,text] of [['packages/contracts/schemas/readiness.json',JSON.stringify(schemas,null,2)+'\n'],['packages/contracts/src/generated.ts',types],['packages/contracts/src/validators.mjs',validators],['packages/contracts/src/validators.d.mts',declarations]]){
  if(process.argv.includes('--check')){if(!fs.existsSync(file)||fs.readFileSync(file,'utf8').replace(/\r\n/g,'\n')!==text)throw Error('Generated contract drift: '+file);}
  else {fs.mkdirSync(file.slice(0,file.lastIndexOf('/')),{recursive:true});fs.writeFileSync(file,text);}
}
