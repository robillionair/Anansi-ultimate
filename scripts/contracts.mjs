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
const ajv=new Ajv2020({strict:false,formats:{uint8:true,uint64:true},code:{source:true,esm:true}});
for(const [name,schema] of Object.entries(schemas))ajv.addSchema(schema,name);
const validators=standaloneCode(ajv,{validReadiness:'Readiness',validError:'CommandError',validReadinessRequest:'ReadinessRequest'})+'\n';
const declarations="// Generated from Rust schemas.\nimport type {Readiness,CommandError,ReadinessRequest} from './generated';\nexport function validReadiness(value:unknown):value is Readiness;\nexport function validError(value:unknown):value is CommandError;\nexport function validReadinessRequest(value:unknown):value is ReadinessRequest;\n";
for(const [file,text] of [['packages/contracts/schemas/readiness.json',JSON.stringify(schemas,null,2)+'\n'],['packages/contracts/src/generated.ts',types],['packages/contracts/src/validators.mjs',validators],['packages/contracts/src/validators.d.mts',declarations]]){
  if(process.argv.includes('--check')){if(!fs.existsSync(file)||fs.readFileSync(file,'utf8').replace(/\r\n/g,'\n')!==text)throw Error('Generated contract drift: '+file);}
  else {fs.mkdirSync(file.slice(0,file.lastIndexOf('/')),{recursive:true});fs.writeFileSync(file,text);}
}
