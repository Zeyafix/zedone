"use strict";
export const CommandResponse_AccessSnapshot = validate21;
const schema32 = {"oneOf":[{"$ref":"#/$defs/CommandSuccess_AccessSnapshot"},{"$ref":"#/$defs/CommandFailure"}],"description":"Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.\nA success confirms the original completion, not the current object state.\nConstruct success only AFTER the storage transaction commits. Cancellation,\na lost response or uncertain commit must never be converted to success."};
const schema33 = {"type":"object","required":["status","result","replayed"],"properties":{"replayed":{"type":"boolean"},"result":{"type":"object","description":"No keys or verifiers leave storage. Version includes retired/current key\nidentities internally so a regrant cannot revive a stale revocation request.","required":["subject","scope","revoked","version"],"properties":{"expires_at_unix_ms":{"type":["integer","null"],"format":"int64"},"revoked":{"type":"boolean"},"scope":{"$ref":"#/$defs/AccessScope"},"subject":{"type":"string"},"version":{"$ref":"#/$defs/StateVersion"}},"additionalProperties":false},"status":{"$ref":"#/$defs/SuccessStatus"}},"additionalProperties":false};
const schema43 = {"type":"array","items":{"type":"integer","maximum":255,"minimum":0},"maxItems":32,"minItems":32};
const schema44 = {"type":"string","enum":["ok"]};
const schema34 = {"oneOf":[{"$ref":"#/$defs/AdministrationScope"},{"$ref":"#/$defs/AdministrationAndSpacesScope"},{"$ref":"#/$defs/SpacesScope"}]};
const schema35 = {"type":"object","required":["kind"],"properties":{"kind":{"$ref":"#/$defs/AdministrationKind"}},"additionalProperties":false};
const schema36 = {"type":"string","enum":["administration"]};

function validate24(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate24.evaluated;
if(evaluated0.dynamicProps){
evaluated0.props = undefined;
}
if(evaluated0.dynamicItems){
evaluated0.items = undefined;
}
if(errors === 0){
if(data && typeof data == "object" && !Array.isArray(data)){
let missing0;
if((data.kind === undefined) && (missing0 = "kind")){
validate24.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(key0 === "kind")){
validate24.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.kind !== undefined){
let data0 = data.kind;
if(typeof data0 !== "string"){
validate24.errors = [{instancePath:instancePath+"/kind",schemaPath:"#/$defs/AdministrationKind/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data0 === "administration")){
validate24.errors = [{instancePath:instancePath+"/kind",schemaPath:"#/$defs/AdministrationKind/enum",keyword:"enum",params:{allowedValues: schema36.enum},message:"must be equal to one of the allowed values"}];
return false;
}
}
}
}
}
else {
validate24.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate24.errors = vErrors;
return errors === 0;
}
validate24.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};

const schema37 = {"type":"object","required":["kind","spaces"],"properties":{"kind":{"$ref":"#/$defs/AdministrationAndSpacesKind"},"spaces":{"type":"object","additionalProperties":{"$ref":"#/$defs/AccessMode"},"propertyNames":{"type":"string"}}},"additionalProperties":false};
const schema38 = {"type":"string","enum":["administration_and_spaces"]};
const schema39 = {"type":"string","enum":["read","read_write"]};

function validate26(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate26.evaluated;
if(evaluated0.dynamicProps){
evaluated0.props = undefined;
}
if(evaluated0.dynamicItems){
evaluated0.items = undefined;
}
if(errors === 0){
if(data && typeof data == "object" && !Array.isArray(data)){
let missing0;
if(((data.kind === undefined) && (missing0 = "kind")) || ((data.spaces === undefined) && (missing0 = "spaces"))){
validate26.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!((key0 === "kind") || (key0 === "spaces"))){
validate26.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.kind !== undefined){
let data0 = data.kind;
const _errs2 = errors;
if(typeof data0 !== "string"){
validate26.errors = [{instancePath:instancePath+"/kind",schemaPath:"#/$defs/AdministrationAndSpacesKind/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data0 === "administration_and_spaces")){
validate26.errors = [{instancePath:instancePath+"/kind",schemaPath:"#/$defs/AdministrationAndSpacesKind/enum",keyword:"enum",params:{allowedValues: schema38.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs2 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.spaces !== undefined){
let data1 = data.spaces;
const _errs5 = errors;
if(errors === _errs5){
if(data1 && typeof data1 == "object" && !Array.isArray(data1)){
for(const key1 in data1){
const _errs7 = errors;
if(typeof key1 !== "string"){
const err0 = {instancePath:instancePath+"/spaces",schemaPath:"#/properties/spaces/propertyNames/type",keyword:"type",params:{type: "string"},message:"must be string",propertyName:key1};
if(vErrors === null){
vErrors = [err0];
}
else {
vErrors.push(err0);
}
errors++;
}
var valid2 = _errs7 === errors;
if(!valid2){
const err1 = {instancePath:instancePath+"/spaces",schemaPath:"#/properties/spaces/propertyNames",keyword:"propertyNames",params:{propertyName: key1},message:"property name must be valid"};
if(vErrors === null){
vErrors = [err1];
}
else {
vErrors.push(err1);
}
errors++;
validate26.errors = vErrors;
return false;
break;
}
}
if(valid2){
for(const key2 in data1){
let data2 = data1[key2];
const _errs10 = errors;
if(typeof data2 !== "string"){
validate26.errors = [{instancePath:instancePath+"/spaces/" + key2.replace(/~/g, "~0").replace(/\//g, "~1"),schemaPath:"#/$defs/AccessMode/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!((data2 === "read") || (data2 === "read_write"))){
validate26.errors = [{instancePath:instancePath+"/spaces/" + key2.replace(/~/g, "~0").replace(/\//g, "~1"),schemaPath:"#/$defs/AccessMode/enum",keyword:"enum",params:{allowedValues: schema39.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid3 = _errs10 === errors;
if(!valid3){
break;
}
}
}
}
else {
validate26.errors = [{instancePath:instancePath+"/spaces",schemaPath:"#/properties/spaces/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
var valid0 = _errs5 === errors;
}
else {
var valid0 = true;
}
}
}
}
}
else {
validate26.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate26.errors = vErrors;
return errors === 0;
}
validate26.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};

const schema40 = {"type":"object","required":["kind","spaces"],"properties":{"kind":{"$ref":"#/$defs/SpacesKind"},"spaces":{"type":"object","additionalProperties":{"$ref":"#/$defs/AccessMode"},"propertyNames":{"type":"string"}}},"additionalProperties":false};
const schema41 = {"type":"string","enum":["spaces"]};

function validate28(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate28.evaluated;
if(evaluated0.dynamicProps){
evaluated0.props = undefined;
}
if(evaluated0.dynamicItems){
evaluated0.items = undefined;
}
if(errors === 0){
if(data && typeof data == "object" && !Array.isArray(data)){
let missing0;
if(((data.kind === undefined) && (missing0 = "kind")) || ((data.spaces === undefined) && (missing0 = "spaces"))){
validate28.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!((key0 === "kind") || (key0 === "spaces"))){
validate28.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.kind !== undefined){
let data0 = data.kind;
const _errs2 = errors;
if(typeof data0 !== "string"){
validate28.errors = [{instancePath:instancePath+"/kind",schemaPath:"#/$defs/SpacesKind/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data0 === "spaces")){
validate28.errors = [{instancePath:instancePath+"/kind",schemaPath:"#/$defs/SpacesKind/enum",keyword:"enum",params:{allowedValues: schema41.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs2 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.spaces !== undefined){
let data1 = data.spaces;
const _errs5 = errors;
if(errors === _errs5){
if(data1 && typeof data1 == "object" && !Array.isArray(data1)){
for(const key1 in data1){
const _errs7 = errors;
if(typeof key1 !== "string"){
const err0 = {instancePath:instancePath+"/spaces",schemaPath:"#/properties/spaces/propertyNames/type",keyword:"type",params:{type: "string"},message:"must be string",propertyName:key1};
if(vErrors === null){
vErrors = [err0];
}
else {
vErrors.push(err0);
}
errors++;
}
var valid2 = _errs7 === errors;
if(!valid2){
const err1 = {instancePath:instancePath+"/spaces",schemaPath:"#/properties/spaces/propertyNames",keyword:"propertyNames",params:{propertyName: key1},message:"property name must be valid"};
if(vErrors === null){
vErrors = [err1];
}
else {
vErrors.push(err1);
}
errors++;
validate28.errors = vErrors;
return false;
break;
}
}
if(valid2){
for(const key2 in data1){
let data2 = data1[key2];
const _errs10 = errors;
if(typeof data2 !== "string"){
validate28.errors = [{instancePath:instancePath+"/spaces/" + key2.replace(/~/g, "~0").replace(/\//g, "~1"),schemaPath:"#/$defs/AccessMode/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!((data2 === "read") || (data2 === "read_write"))){
validate28.errors = [{instancePath:instancePath+"/spaces/" + key2.replace(/~/g, "~0").replace(/\//g, "~1"),schemaPath:"#/$defs/AccessMode/enum",keyword:"enum",params:{allowedValues: schema39.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid3 = _errs10 === errors;
if(!valid3){
break;
}
}
}
}
else {
validate28.errors = [{instancePath:instancePath+"/spaces",schemaPath:"#/properties/spaces/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
var valid0 = _errs5 === errors;
}
else {
var valid0 = true;
}
}
}
}
}
else {
validate28.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate28.errors = vErrors;
return errors === 0;
}
validate28.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate23(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate23.evaluated;
if(evaluated0.dynamicProps){
evaluated0.props = undefined;
}
if(evaluated0.dynamicItems){
evaluated0.items = undefined;
}
const _errs0 = errors;
let valid0 = false;
let passing0 = null;
const _errs1 = errors;
if(!(validate24(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate24.errors : vErrors.concat(validate24.errors);
errors = vErrors.length;
}
var _valid0 = _errs1 === errors;
if(_valid0){
valid0 = true;
passing0 = 0;
var props0 = true;
}
const _errs2 = errors;
if(!(validate26(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate26.errors : vErrors.concat(validate26.errors);
errors = vErrors.length;
}
var _valid0 = _errs2 === errors;
if(_valid0 && valid0){
valid0 = false;
passing0 = [passing0, 1];
}
else {
if(_valid0){
valid0 = true;
passing0 = 1;
if(props0 !== true){
props0 = true;
}
}
const _errs3 = errors;
if(!(validate28(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate28.errors : vErrors.concat(validate28.errors);
errors = vErrors.length;
}
var _valid0 = _errs3 === errors;
if(_valid0 && valid0){
valid0 = false;
passing0 = [passing0, 2];
}
else {
if(_valid0){
valid0 = true;
passing0 = 2;
if(props0 !== true){
props0 = true;
}
}
}
}
if(!valid0){
const err0 = {instancePath,schemaPath:"#/oneOf",keyword:"oneOf",params:{passingSchemas: passing0},message:"must match exactly one schema in oneOf"};
if(vErrors === null){
vErrors = [err0];
}
else {
vErrors.push(err0);
}
errors++;
validate23.errors = vErrors;
return false;
}
else {
errors = _errs0;
if(vErrors !== null){
if(_errs0){
vErrors.length = _errs0;
}
else {
vErrors = null;
}
}
}
validate23.errors = vErrors;
evaluated0.props = props0;
return errors === 0;
}
validate23.evaluated = {"dynamicProps":true,"dynamicItems":false};


function validate22(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate22.evaluated;
if(evaluated0.dynamicProps){
evaluated0.props = undefined;
}
if(evaluated0.dynamicItems){
evaluated0.items = undefined;
}
if(errors === 0){
if(data && typeof data == "object" && !Array.isArray(data)){
let missing0;
if((((data.status === undefined) && (missing0 = "status")) || ((data.result === undefined) && (missing0 = "result"))) || ((data.replayed === undefined) && (missing0 = "replayed"))){
validate22.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((key0 === "replayed") || (key0 === "result")) || (key0 === "status"))){
validate22.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.replayed !== undefined){
const _errs2 = errors;
if(typeof data.replayed !== "boolean"){
validate22.errors = [{instancePath:instancePath+"/replayed",schemaPath:"#/properties/replayed/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid0 = _errs2 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.result !== undefined){
let data1 = data.result;
const _errs4 = errors;
if(errors === _errs4){
if(data1 && typeof data1 == "object" && !Array.isArray(data1)){
let missing1;
if(((((data1.subject === undefined) && (missing1 = "subject")) || ((data1.scope === undefined) && (missing1 = "scope"))) || ((data1.revoked === undefined) && (missing1 = "revoked"))) || ((data1.version === undefined) && (missing1 = "version"))){
validate22.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/required",keyword:"required",params:{missingProperty: missing1},message:"must have required property '"+missing1+"'"}];
return false;
}
else {
const _errs6 = errors;
for(const key1 in data1){
if(!(((((key1 === "expires_at_unix_ms") || (key1 === "revoked")) || (key1 === "scope")) || (key1 === "subject")) || (key1 === "version"))){
validate22.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key1},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs6 === errors){
if(data1.expires_at_unix_ms !== undefined){
let data2 = data1.expires_at_unix_ms;
const _errs7 = errors;
if((!((typeof data2 == "number") && (!(data2 % 1) && !isNaN(data2)))) && (data2 !== null)){
validate22.errors = [{instancePath:instancePath+"/result/expires_at_unix_ms",schemaPath:"#/properties/result/properties/expires_at_unix_ms/type",keyword:"type",params:{type: schema33.properties.result.properties.expires_at_unix_ms.type},message:"must be integer,null"}];
return false;
}
var valid1 = _errs7 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.revoked !== undefined){
const _errs9 = errors;
if(typeof data1.revoked !== "boolean"){
validate22.errors = [{instancePath:instancePath+"/result/revoked",schemaPath:"#/properties/result/properties/revoked/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid1 = _errs9 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.scope !== undefined){
const _errs11 = errors;
if(!(validate23(data1.scope, {instancePath:instancePath+"/result/scope",parentData:data1,parentDataProperty:"scope",rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate23.errors : vErrors.concat(validate23.errors);
errors = vErrors.length;
}
var valid1 = _errs11 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.subject !== undefined){
const _errs12 = errors;
if(typeof data1.subject !== "string"){
validate22.errors = [{instancePath:instancePath+"/result/subject",schemaPath:"#/properties/result/properties/subject/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid1 = _errs12 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.version !== undefined){
let data6 = data1.version;
const _errs14 = errors;
const _errs15 = errors;
if(errors === _errs15){
if(Array.isArray(data6)){
if(data6.length > 32){
validate22.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/maxItems",keyword:"maxItems",params:{limit: 32},message:"must NOT have more than 32 items"}];
return false;
}
else {
if(data6.length < 32){
validate22.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/minItems",keyword:"minItems",params:{limit: 32},message:"must NOT have fewer than 32 items"}];
return false;
}
else {
var valid3 = true;
const len0 = data6.length;
for(let i0=0; i0<len0; i0++){
let data7 = data6[i0];
const _errs17 = errors;
if(!((typeof data7 == "number") && (!(data7 % 1) && !isNaN(data7)))){
validate22.errors = [{instancePath:instancePath+"/result/version/" + i0,schemaPath:"#/$defs/StateVersion/items/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs17){
if(typeof data7 == "number"){
if(data7 > 255 || isNaN(data7)){
validate22.errors = [{instancePath:instancePath+"/result/version/" + i0,schemaPath:"#/$defs/StateVersion/items/maximum",keyword:"maximum",params:{comparison: "<=", limit: 255},message:"must be <= 255"}];
return false;
}
else {
if(data7 < 0 || isNaN(data7)){
validate22.errors = [{instancePath:instancePath+"/result/version/" + i0,schemaPath:"#/$defs/StateVersion/items/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
}
var valid3 = _errs17 === errors;
if(!valid3){
break;
}
}
}
}
}
else {
validate22.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs14 === errors;
}
else {
var valid1 = true;
}
}
}
}
}
}
}
}
else {
validate22.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
var valid0 = _errs4 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.status !== undefined){
let data8 = data.status;
const _errs19 = errors;
if(typeof data8 !== "string"){
validate22.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data8 === "ok")){
validate22.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/enum",keyword:"enum",params:{allowedValues: schema44.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs19 === errors;
}
else {
var valid0 = true;
}
}
}
}
}
}
else {
validate22.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate22.errors = vErrors;
return errors === 0;
}
validate22.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};

const schema45 = {"type":"object","required":["status","error"],"properties":{"error":{"$ref":"#/$defs/CommandError"},"status":{"$ref":"#/$defs/ErrorStatus"}},"additionalProperties":false};
const schema48 = {"type":"string","enum":["error"]};
const schema46 = {"type":"object","required":["code"],"properties":{"code":{"$ref":"#/$defs/ErrorCode"}},"additionalProperties":false};
const schema47 = {"type":"string","description":"Shared machine-readable errors, without credentials, SQL or platform details.\n\nCodes describe semantics, not HTTP statuses or retry policy. A stale request\nrequires a fresh read and a new explicit action. An unknown outcome requires\nchecking/replaying the original command, never inventing a new command ID.","enum":["invalid_request","access_denied","not_found","stale_state","conflict","command_mismatch","unavailable","outcome_unknown"]};

function validate33(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate33.evaluated;
if(evaluated0.dynamicProps){
evaluated0.props = undefined;
}
if(evaluated0.dynamicItems){
evaluated0.items = undefined;
}
if(errors === 0){
if(data && typeof data == "object" && !Array.isArray(data)){
let missing0;
if((data.code === undefined) && (missing0 = "code")){
validate33.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(key0 === "code")){
validate33.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.code !== undefined){
let data0 = data.code;
if(typeof data0 !== "string"){
validate33.errors = [{instancePath:instancePath+"/code",schemaPath:"#/$defs/ErrorCode/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!((((((((data0 === "invalid_request") || (data0 === "access_denied")) || (data0 === "not_found")) || (data0 === "stale_state")) || (data0 === "conflict")) || (data0 === "command_mismatch")) || (data0 === "unavailable")) || (data0 === "outcome_unknown"))){
validate33.errors = [{instancePath:instancePath+"/code",schemaPath:"#/$defs/ErrorCode/enum",keyword:"enum",params:{allowedValues: schema47.enum},message:"must be equal to one of the allowed values"}];
return false;
}
}
}
}
}
else {
validate33.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate33.errors = vErrors;
return errors === 0;
}
validate33.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate32(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate32.evaluated;
if(evaluated0.dynamicProps){
evaluated0.props = undefined;
}
if(evaluated0.dynamicItems){
evaluated0.items = undefined;
}
if(errors === 0){
if(data && typeof data == "object" && !Array.isArray(data)){
let missing0;
if(((data.status === undefined) && (missing0 = "status")) || ((data.error === undefined) && (missing0 = "error"))){
validate32.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!((key0 === "error") || (key0 === "status"))){
validate32.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.error !== undefined){
const _errs2 = errors;
if(!(validate33(data.error, {instancePath:instancePath+"/error",parentData:data,parentDataProperty:"error",rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate33.errors : vErrors.concat(validate33.errors);
errors = vErrors.length;
}
var valid0 = _errs2 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.status !== undefined){
let data1 = data.status;
const _errs3 = errors;
if(typeof data1 !== "string"){
validate32.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/ErrorStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data1 === "error")){
validate32.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/ErrorStatus/enum",keyword:"enum",params:{allowedValues: schema48.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs3 === errors;
}
else {
var valid0 = true;
}
}
}
}
}
else {
validate32.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate32.errors = vErrors;
return errors === 0;
}
validate32.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate21(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate21.evaluated;
if(evaluated0.dynamicProps){
evaluated0.props = undefined;
}
if(evaluated0.dynamicItems){
evaluated0.items = undefined;
}
const _errs0 = errors;
let valid0 = false;
let passing0 = null;
const _errs1 = errors;
if(!(validate22(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate22.errors : vErrors.concat(validate22.errors);
errors = vErrors.length;
}
var _valid0 = _errs1 === errors;
if(_valid0){
valid0 = true;
passing0 = 0;
var props0 = true;
}
const _errs2 = errors;
if(!(validate32(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate32.errors : vErrors.concat(validate32.errors);
errors = vErrors.length;
}
var _valid0 = _errs2 === errors;
if(_valid0 && valid0){
valid0 = false;
passing0 = [passing0, 1];
}
else {
if(_valid0){
valid0 = true;
passing0 = 1;
if(props0 !== true){
props0 = true;
}
}
}
if(!valid0){
const err0 = {instancePath,schemaPath:"#/oneOf",keyword:"oneOf",params:{passingSchemas: passing0},message:"must match exactly one schema in oneOf"};
if(vErrors === null){
vErrors = [err0];
}
else {
vErrors.push(err0);
}
errors++;
validate21.errors = vErrors;
return false;
}
else {
errors = _errs0;
if(vErrors !== null){
if(_errs0){
vErrors.length = _errs0;
}
else {
vErrors = null;
}
}
}
validate21.errors = vErrors;
evaluated0.props = props0;
return errors === 0;
}
validate21.evaluated = {"dynamicProps":true,"dynamicItems":false};

export const CommandResponse_CardSnapshot = validate36;
const schema49 = {"oneOf":[{"$ref":"#/$defs/CommandSuccess_CardSnapshot"},{"$ref":"#/$defs/CommandFailure"}],"description":"Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.\nA success confirms the original completion, not the current object state.\nConstruct success only AFTER the storage transaction commits. Cancellation,\na lost response or uncertain commit must never be converted to success."};
const schema50 = {"type":"object","required":["status","result","replayed"],"properties":{"replayed":{"type":"boolean"},"result":{"type":"object","required":["space","card","titles","descriptions","columns","positions","completed","tags","state","deletion_groups","version"],"properties":{"card":{"type":"string"},"columns":{"type":"array","items":{"type":"string"}},"completed":{"type":"array","items":{"type":"boolean"}},"deletion_groups":{"type":"array","items":{"type":"string"}},"descriptions":{"type":"array","items":{"type":"string"}},"positions":{"type":"array","items":{"type":"integer","format":"int64","minimum":0}},"space":{"type":"string"},"state":{"$ref":"#/$defs/SpaceState"},"tags":{"type":"array","items":{"type":"string"}},"titles":{"type":"array","items":{"type":"string"}},"version":{"$ref":"#/$defs/StateVersion"}},"additionalProperties":false},"status":{"$ref":"#/$defs/SuccessStatus"}},"additionalProperties":false};
const schema51 = {"type":"string","enum":["active","trashed","conflicted"]};
const func1 = Object.prototype.hasOwnProperty;

function validate37(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate37.evaluated;
if(evaluated0.dynamicProps){
evaluated0.props = undefined;
}
if(evaluated0.dynamicItems){
evaluated0.items = undefined;
}
if(errors === 0){
if(data && typeof data == "object" && !Array.isArray(data)){
let missing0;
if((((data.status === undefined) && (missing0 = "status")) || ((data.result === undefined) && (missing0 = "result"))) || ((data.replayed === undefined) && (missing0 = "replayed"))){
validate37.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((key0 === "replayed") || (key0 === "result")) || (key0 === "status"))){
validate37.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.replayed !== undefined){
const _errs2 = errors;
if(typeof data.replayed !== "boolean"){
validate37.errors = [{instancePath:instancePath+"/replayed",schemaPath:"#/properties/replayed/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid0 = _errs2 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.result !== undefined){
let data1 = data.result;
const _errs4 = errors;
if(errors === _errs4){
if(data1 && typeof data1 == "object" && !Array.isArray(data1)){
let missing1;
if((((((((((((data1.space === undefined) && (missing1 = "space")) || ((data1.card === undefined) && (missing1 = "card"))) || ((data1.titles === undefined) && (missing1 = "titles"))) || ((data1.descriptions === undefined) && (missing1 = "descriptions"))) || ((data1.columns === undefined) && (missing1 = "columns"))) || ((data1.positions === undefined) && (missing1 = "positions"))) || ((data1.completed === undefined) && (missing1 = "completed"))) || ((data1.tags === undefined) && (missing1 = "tags"))) || ((data1.state === undefined) && (missing1 = "state"))) || ((data1.deletion_groups === undefined) && (missing1 = "deletion_groups"))) || ((data1.version === undefined) && (missing1 = "version"))){
validate37.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/required",keyword:"required",params:{missingProperty: missing1},message:"must have required property '"+missing1+"'"}];
return false;
}
else {
const _errs6 = errors;
for(const key1 in data1){
if(!(func1.call(schema50.properties.result.properties, key1))){
validate37.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key1},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs6 === errors){
if(data1.card !== undefined){
const _errs7 = errors;
if(typeof data1.card !== "string"){
validate37.errors = [{instancePath:instancePath+"/result/card",schemaPath:"#/properties/result/properties/card/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid1 = _errs7 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.columns !== undefined){
let data3 = data1.columns;
const _errs9 = errors;
if(errors === _errs9){
if(Array.isArray(data3)){
var valid2 = true;
const len0 = data3.length;
for(let i0=0; i0<len0; i0++){
const _errs11 = errors;
if(typeof data3[i0] !== "string"){
validate37.errors = [{instancePath:instancePath+"/result/columns/" + i0,schemaPath:"#/properties/result/properties/columns/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid2 = _errs11 === errors;
if(!valid2){
break;
}
}
}
else {
validate37.errors = [{instancePath:instancePath+"/result/columns",schemaPath:"#/properties/result/properties/columns/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs9 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.completed !== undefined){
let data5 = data1.completed;
const _errs13 = errors;
if(errors === _errs13){
if(Array.isArray(data5)){
var valid3 = true;
const len1 = data5.length;
for(let i1=0; i1<len1; i1++){
const _errs15 = errors;
if(typeof data5[i1] !== "boolean"){
validate37.errors = [{instancePath:instancePath+"/result/completed/" + i1,schemaPath:"#/properties/result/properties/completed/items/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid3 = _errs15 === errors;
if(!valid3){
break;
}
}
}
else {
validate37.errors = [{instancePath:instancePath+"/result/completed",schemaPath:"#/properties/result/properties/completed/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs13 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.deletion_groups !== undefined){
let data7 = data1.deletion_groups;
const _errs17 = errors;
if(errors === _errs17){
if(Array.isArray(data7)){
var valid4 = true;
const len2 = data7.length;
for(let i2=0; i2<len2; i2++){
const _errs19 = errors;
if(typeof data7[i2] !== "string"){
validate37.errors = [{instancePath:instancePath+"/result/deletion_groups/" + i2,schemaPath:"#/properties/result/properties/deletion_groups/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid4 = _errs19 === errors;
if(!valid4){
break;
}
}
}
else {
validate37.errors = [{instancePath:instancePath+"/result/deletion_groups",schemaPath:"#/properties/result/properties/deletion_groups/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs17 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.descriptions !== undefined){
let data9 = data1.descriptions;
const _errs21 = errors;
if(errors === _errs21){
if(Array.isArray(data9)){
var valid5 = true;
const len3 = data9.length;
for(let i3=0; i3<len3; i3++){
const _errs23 = errors;
if(typeof data9[i3] !== "string"){
validate37.errors = [{instancePath:instancePath+"/result/descriptions/" + i3,schemaPath:"#/properties/result/properties/descriptions/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid5 = _errs23 === errors;
if(!valid5){
break;
}
}
}
else {
validate37.errors = [{instancePath:instancePath+"/result/descriptions",schemaPath:"#/properties/result/properties/descriptions/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs21 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.positions !== undefined){
let data11 = data1.positions;
const _errs25 = errors;
if(errors === _errs25){
if(Array.isArray(data11)){
var valid6 = true;
const len4 = data11.length;
for(let i4=0; i4<len4; i4++){
let data12 = data11[i4];
const _errs27 = errors;
if(!((typeof data12 == "number") && (!(data12 % 1) && !isNaN(data12)))){
validate37.errors = [{instancePath:instancePath+"/result/positions/" + i4,schemaPath:"#/properties/result/properties/positions/items/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs27){
if(typeof data12 == "number"){
if(data12 < 0 || isNaN(data12)){
validate37.errors = [{instancePath:instancePath+"/result/positions/" + i4,schemaPath:"#/properties/result/properties/positions/items/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
var valid6 = _errs27 === errors;
if(!valid6){
break;
}
}
}
else {
validate37.errors = [{instancePath:instancePath+"/result/positions",schemaPath:"#/properties/result/properties/positions/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs25 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.space !== undefined){
const _errs29 = errors;
if(typeof data1.space !== "string"){
validate37.errors = [{instancePath:instancePath+"/result/space",schemaPath:"#/properties/result/properties/space/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid1 = _errs29 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.state !== undefined){
let data14 = data1.state;
const _errs31 = errors;
if(typeof data14 !== "string"){
validate37.errors = [{instancePath:instancePath+"/result/state",schemaPath:"#/$defs/SpaceState/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(((data14 === "active") || (data14 === "trashed")) || (data14 === "conflicted"))){
validate37.errors = [{instancePath:instancePath+"/result/state",schemaPath:"#/$defs/SpaceState/enum",keyword:"enum",params:{allowedValues: schema51.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid1 = _errs31 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.tags !== undefined){
let data15 = data1.tags;
const _errs34 = errors;
if(errors === _errs34){
if(Array.isArray(data15)){
var valid8 = true;
const len5 = data15.length;
for(let i5=0; i5<len5; i5++){
const _errs36 = errors;
if(typeof data15[i5] !== "string"){
validate37.errors = [{instancePath:instancePath+"/result/tags/" + i5,schemaPath:"#/properties/result/properties/tags/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid8 = _errs36 === errors;
if(!valid8){
break;
}
}
}
else {
validate37.errors = [{instancePath:instancePath+"/result/tags",schemaPath:"#/properties/result/properties/tags/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs34 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.titles !== undefined){
let data17 = data1.titles;
const _errs38 = errors;
if(errors === _errs38){
if(Array.isArray(data17)){
var valid9 = true;
const len6 = data17.length;
for(let i6=0; i6<len6; i6++){
const _errs40 = errors;
if(typeof data17[i6] !== "string"){
validate37.errors = [{instancePath:instancePath+"/result/titles/" + i6,schemaPath:"#/properties/result/properties/titles/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid9 = _errs40 === errors;
if(!valid9){
break;
}
}
}
else {
validate37.errors = [{instancePath:instancePath+"/result/titles",schemaPath:"#/properties/result/properties/titles/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs38 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.version !== undefined){
let data19 = data1.version;
const _errs42 = errors;
const _errs43 = errors;
if(errors === _errs43){
if(Array.isArray(data19)){
if(data19.length > 32){
validate37.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/maxItems",keyword:"maxItems",params:{limit: 32},message:"must NOT have more than 32 items"}];
return false;
}
else {
if(data19.length < 32){
validate37.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/minItems",keyword:"minItems",params:{limit: 32},message:"must NOT have fewer than 32 items"}];
return false;
}
else {
var valid11 = true;
const len7 = data19.length;
for(let i7=0; i7<len7; i7++){
let data20 = data19[i7];
const _errs45 = errors;
if(!((typeof data20 == "number") && (!(data20 % 1) && !isNaN(data20)))){
validate37.errors = [{instancePath:instancePath+"/result/version/" + i7,schemaPath:"#/$defs/StateVersion/items/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs45){
if(typeof data20 == "number"){
if(data20 > 255 || isNaN(data20)){
validate37.errors = [{instancePath:instancePath+"/result/version/" + i7,schemaPath:"#/$defs/StateVersion/items/maximum",keyword:"maximum",params:{comparison: "<=", limit: 255},message:"must be <= 255"}];
return false;
}
else {
if(data20 < 0 || isNaN(data20)){
validate37.errors = [{instancePath:instancePath+"/result/version/" + i7,schemaPath:"#/$defs/StateVersion/items/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
}
var valid11 = _errs45 === errors;
if(!valid11){
break;
}
}
}
}
}
else {
validate37.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs42 === errors;
}
else {
var valid1 = true;
}
}
}
}
}
}
}
}
}
}
}
}
}
}
else {
validate37.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
var valid0 = _errs4 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.status !== undefined){
let data21 = data.status;
const _errs47 = errors;
if(typeof data21 !== "string"){
validate37.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data21 === "ok")){
validate37.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/enum",keyword:"enum",params:{allowedValues: schema44.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs47 === errors;
}
else {
var valid0 = true;
}
}
}
}
}
}
else {
validate37.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate37.errors = vErrors;
return errors === 0;
}
validate37.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate36(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate36.evaluated;
if(evaluated0.dynamicProps){
evaluated0.props = undefined;
}
if(evaluated0.dynamicItems){
evaluated0.items = undefined;
}
const _errs0 = errors;
let valid0 = false;
let passing0 = null;
const _errs1 = errors;
if(!(validate37(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate37.errors : vErrors.concat(validate37.errors);
errors = vErrors.length;
}
var _valid0 = _errs1 === errors;
if(_valid0){
valid0 = true;
passing0 = 0;
var props0 = true;
}
const _errs2 = errors;
if(!(validate32(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate32.errors : vErrors.concat(validate32.errors);
errors = vErrors.length;
}
var _valid0 = _errs2 === errors;
if(_valid0 && valid0){
valid0 = false;
passing0 = [passing0, 1];
}
else {
if(_valid0){
valid0 = true;
passing0 = 1;
if(props0 !== true){
props0 = true;
}
}
}
if(!valid0){
const err0 = {instancePath,schemaPath:"#/oneOf",keyword:"oneOf",params:{passingSchemas: passing0},message:"must match exactly one schema in oneOf"};
if(vErrors === null){
vErrors = [err0];
}
else {
vErrors.push(err0);
}
errors++;
validate36.errors = vErrors;
return false;
}
else {
errors = _errs0;
if(vErrors !== null){
if(_errs0){
vErrors.length = _errs0;
}
else {
vErrors = null;
}
}
}
validate36.errors = vErrors;
evaluated0.props = props0;
return errors === 0;
}
validate36.evaluated = {"dynamicProps":true,"dynamicItems":false};

export const CommandResponse_CardHistory = validate40;
const schema54 = {"oneOf":[{"$ref":"#/$defs/CommandSuccess_CardHistory"},{"$ref":"#/$defs/CommandFailure"}],"description":"Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.\nA success confirms the original completion, not the current object state.\nConstruct success only AFTER the storage transaction commits. Cancellation,\na lost response or uncertain commit must never be converted to success."};
const schema55 = {"type":"object","required":["status","result","replayed"],"properties":{"replayed":{"type":"boolean"},"result":{"type":"object","required":["space","card","versions","version"],"properties":{"card":{"type":"string"},"next":{"type":["string","null"]},"space":{"type":"string"},"version":{"$ref":"#/$defs/StateVersion"},"versions":{"type":"array","items":{"$ref":"#/$defs/CardTextVersion"}}},"additionalProperties":false},"status":{"$ref":"#/$defs/SuccessStatus"}},"additionalProperties":false};
const schema57 = {"type":"object","required":["id","subject","sequence","title","description"],"properties":{"description":{"type":"string"},"id":{"type":"string"},"sequence":{"type":"integer","format":"int64","description":"Local causal sequence; independent branches may share a sequence.","minimum":0},"subject":{"type":"string"},"title":{"type":"string"}},"additionalProperties":false};

function validate41(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate41.evaluated;
if(evaluated0.dynamicProps){
evaluated0.props = undefined;
}
if(evaluated0.dynamicItems){
evaluated0.items = undefined;
}
if(errors === 0){
if(data && typeof data == "object" && !Array.isArray(data)){
let missing0;
if((((data.status === undefined) && (missing0 = "status")) || ((data.result === undefined) && (missing0 = "result"))) || ((data.replayed === undefined) && (missing0 = "replayed"))){
validate41.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((key0 === "replayed") || (key0 === "result")) || (key0 === "status"))){
validate41.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.replayed !== undefined){
const _errs2 = errors;
if(typeof data.replayed !== "boolean"){
validate41.errors = [{instancePath:instancePath+"/replayed",schemaPath:"#/properties/replayed/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid0 = _errs2 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.result !== undefined){
let data1 = data.result;
const _errs4 = errors;
if(errors === _errs4){
if(data1 && typeof data1 == "object" && !Array.isArray(data1)){
let missing1;
if(((((data1.space === undefined) && (missing1 = "space")) || ((data1.card === undefined) && (missing1 = "card"))) || ((data1.versions === undefined) && (missing1 = "versions"))) || ((data1.version === undefined) && (missing1 = "version"))){
validate41.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/required",keyword:"required",params:{missingProperty: missing1},message:"must have required property '"+missing1+"'"}];
return false;
}
else {
const _errs6 = errors;
for(const key1 in data1){
if(!(((((key1 === "card") || (key1 === "next")) || (key1 === "space")) || (key1 === "version")) || (key1 === "versions"))){
validate41.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key1},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs6 === errors){
if(data1.card !== undefined){
const _errs7 = errors;
if(typeof data1.card !== "string"){
validate41.errors = [{instancePath:instancePath+"/result/card",schemaPath:"#/properties/result/properties/card/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid1 = _errs7 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.next !== undefined){
let data3 = data1.next;
const _errs9 = errors;
if((typeof data3 !== "string") && (data3 !== null)){
validate41.errors = [{instancePath:instancePath+"/result/next",schemaPath:"#/properties/result/properties/next/type",keyword:"type",params:{type: schema55.properties.result.properties.next.type},message:"must be string,null"}];
return false;
}
var valid1 = _errs9 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.space !== undefined){
const _errs11 = errors;
if(typeof data1.space !== "string"){
validate41.errors = [{instancePath:instancePath+"/result/space",schemaPath:"#/properties/result/properties/space/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid1 = _errs11 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.version !== undefined){
let data5 = data1.version;
const _errs13 = errors;
const _errs14 = errors;
if(errors === _errs14){
if(Array.isArray(data5)){
if(data5.length > 32){
validate41.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/maxItems",keyword:"maxItems",params:{limit: 32},message:"must NOT have more than 32 items"}];
return false;
}
else {
if(data5.length < 32){
validate41.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/minItems",keyword:"minItems",params:{limit: 32},message:"must NOT have fewer than 32 items"}];
return false;
}
else {
var valid3 = true;
const len0 = data5.length;
for(let i0=0; i0<len0; i0++){
let data6 = data5[i0];
const _errs16 = errors;
if(!((typeof data6 == "number") && (!(data6 % 1) && !isNaN(data6)))){
validate41.errors = [{instancePath:instancePath+"/result/version/" + i0,schemaPath:"#/$defs/StateVersion/items/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs16){
if(typeof data6 == "number"){
if(data6 > 255 || isNaN(data6)){
validate41.errors = [{instancePath:instancePath+"/result/version/" + i0,schemaPath:"#/$defs/StateVersion/items/maximum",keyword:"maximum",params:{comparison: "<=", limit: 255},message:"must be <= 255"}];
return false;
}
else {
if(data6 < 0 || isNaN(data6)){
validate41.errors = [{instancePath:instancePath+"/result/version/" + i0,schemaPath:"#/$defs/StateVersion/items/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
}
var valid3 = _errs16 === errors;
if(!valid3){
break;
}
}
}
}
}
else {
validate41.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs13 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.versions !== undefined){
let data7 = data1.versions;
const _errs18 = errors;
if(errors === _errs18){
if(Array.isArray(data7)){
var valid4 = true;
const len1 = data7.length;
for(let i1=0; i1<len1; i1++){
let data8 = data7[i1];
const _errs20 = errors;
const _errs21 = errors;
if(errors === _errs21){
if(data8 && typeof data8 == "object" && !Array.isArray(data8)){
let missing2;
if((((((data8.id === undefined) && (missing2 = "id")) || ((data8.subject === undefined) && (missing2 = "subject"))) || ((data8.sequence === undefined) && (missing2 = "sequence"))) || ((data8.title === undefined) && (missing2 = "title"))) || ((data8.description === undefined) && (missing2 = "description"))){
validate41.errors = [{instancePath:instancePath+"/result/versions/" + i1,schemaPath:"#/$defs/CardTextVersion/required",keyword:"required",params:{missingProperty: missing2},message:"must have required property '"+missing2+"'"}];
return false;
}
else {
const _errs23 = errors;
for(const key2 in data8){
if(!(((((key2 === "description") || (key2 === "id")) || (key2 === "sequence")) || (key2 === "subject")) || (key2 === "title"))){
validate41.errors = [{instancePath:instancePath+"/result/versions/" + i1,schemaPath:"#/$defs/CardTextVersion/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key2},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs23 === errors){
if(data8.description !== undefined){
const _errs24 = errors;
if(typeof data8.description !== "string"){
validate41.errors = [{instancePath:instancePath+"/result/versions/" + i1+"/description",schemaPath:"#/$defs/CardTextVersion/properties/description/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid6 = _errs24 === errors;
}
else {
var valid6 = true;
}
if(valid6){
if(data8.id !== undefined){
const _errs26 = errors;
if(typeof data8.id !== "string"){
validate41.errors = [{instancePath:instancePath+"/result/versions/" + i1+"/id",schemaPath:"#/$defs/CardTextVersion/properties/id/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid6 = _errs26 === errors;
}
else {
var valid6 = true;
}
if(valid6){
if(data8.sequence !== undefined){
let data11 = data8.sequence;
const _errs28 = errors;
if(!((typeof data11 == "number") && (!(data11 % 1) && !isNaN(data11)))){
validate41.errors = [{instancePath:instancePath+"/result/versions/" + i1+"/sequence",schemaPath:"#/$defs/CardTextVersion/properties/sequence/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs28){
if(typeof data11 == "number"){
if(data11 < 0 || isNaN(data11)){
validate41.errors = [{instancePath:instancePath+"/result/versions/" + i1+"/sequence",schemaPath:"#/$defs/CardTextVersion/properties/sequence/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
var valid6 = _errs28 === errors;
}
else {
var valid6 = true;
}
if(valid6){
if(data8.subject !== undefined){
const _errs30 = errors;
if(typeof data8.subject !== "string"){
validate41.errors = [{instancePath:instancePath+"/result/versions/" + i1+"/subject",schemaPath:"#/$defs/CardTextVersion/properties/subject/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid6 = _errs30 === errors;
}
else {
var valid6 = true;
}
if(valid6){
if(data8.title !== undefined){
const _errs32 = errors;
if(typeof data8.title !== "string"){
validate41.errors = [{instancePath:instancePath+"/result/versions/" + i1+"/title",schemaPath:"#/$defs/CardTextVersion/properties/title/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid6 = _errs32 === errors;
}
else {
var valid6 = true;
}
}
}
}
}
}
}
}
else {
validate41.errors = [{instancePath:instancePath+"/result/versions/" + i1,schemaPath:"#/$defs/CardTextVersion/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
var valid4 = _errs20 === errors;
if(!valid4){
break;
}
}
}
else {
validate41.errors = [{instancePath:instancePath+"/result/versions",schemaPath:"#/properties/result/properties/versions/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs18 === errors;
}
else {
var valid1 = true;
}
}
}
}
}
}
}
}
else {
validate41.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
var valid0 = _errs4 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.status !== undefined){
let data14 = data.status;
const _errs34 = errors;
if(typeof data14 !== "string"){
validate41.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data14 === "ok")){
validate41.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/enum",keyword:"enum",params:{allowedValues: schema44.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs34 === errors;
}
else {
var valid0 = true;
}
}
}
}
}
}
else {
validate41.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate41.errors = vErrors;
return errors === 0;
}
validate41.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate40(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate40.evaluated;
if(evaluated0.dynamicProps){
evaluated0.props = undefined;
}
if(evaluated0.dynamicItems){
evaluated0.items = undefined;
}
const _errs0 = errors;
let valid0 = false;
let passing0 = null;
const _errs1 = errors;
if(!(validate41(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate41.errors : vErrors.concat(validate41.errors);
errors = vErrors.length;
}
var _valid0 = _errs1 === errors;
if(_valid0){
valid0 = true;
passing0 = 0;
var props0 = true;
}
const _errs2 = errors;
if(!(validate32(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate32.errors : vErrors.concat(validate32.errors);
errors = vErrors.length;
}
var _valid0 = _errs2 === errors;
if(_valid0 && valid0){
valid0 = false;
passing0 = [passing0, 1];
}
else {
if(_valid0){
valid0 = true;
passing0 = 1;
if(props0 !== true){
props0 = true;
}
}
}
if(!valid0){
const err0 = {instancePath,schemaPath:"#/oneOf",keyword:"oneOf",params:{passingSchemas: passing0},message:"must match exactly one schema in oneOf"};
if(vErrors === null){
vErrors = [err0];
}
else {
vErrors.push(err0);
}
errors++;
validate40.errors = vErrors;
return false;
}
else {
errors = _errs0;
if(vErrors !== null){
if(_errs0){
vErrors.length = _errs0;
}
else {
vErrors = null;
}
}
}
validate40.errors = vErrors;
evaluated0.props = props0;
return errors === 0;
}
validate40.evaluated = {"dynamicProps":true,"dynamicItems":false};

export const CommandResponse_CardList = validate44;
const schema59 = {"oneOf":[{"$ref":"#/$defs/CommandSuccess_CardList"},{"$ref":"#/$defs/CommandFailure"}],"description":"Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.\nA success confirms the original completion, not the current object state.\nConstruct success only AFTER the storage transaction commits. Cancellation,\na lost response or uncertain commit must never be converted to success."};
const schema60 = {"type":"object","required":["status","result","replayed"],"properties":{"replayed":{"type":"boolean"},"result":{"type":"object","required":["cards"],"properties":{"cards":{"type":"array","items":{"$ref":"#/$defs/CardSnapshot"}}},"additionalProperties":false},"status":{"$ref":"#/$defs/SuccessStatus"}},"additionalProperties":false};
const schema61 = {"type":"object","required":["space","card","titles","descriptions","columns","positions","completed","tags","state","deletion_groups","version"],"properties":{"card":{"type":"string"},"columns":{"type":"array","items":{"type":"string"}},"completed":{"type":"array","items":{"type":"boolean"}},"deletion_groups":{"type":"array","items":{"type":"string"}},"descriptions":{"type":"array","items":{"type":"string"}},"positions":{"type":"array","items":{"type":"integer","format":"int64","minimum":0}},"space":{"type":"string"},"state":{"$ref":"#/$defs/SpaceState"},"tags":{"type":"array","items":{"type":"string"}},"titles":{"type":"array","items":{"type":"string"}},"version":{"$ref":"#/$defs/StateVersion"}},"additionalProperties":false};

function validate46(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate46.evaluated;
if(evaluated0.dynamicProps){
evaluated0.props = undefined;
}
if(evaluated0.dynamicItems){
evaluated0.items = undefined;
}
if(errors === 0){
if(data && typeof data == "object" && !Array.isArray(data)){
let missing0;
if((((((((((((data.space === undefined) && (missing0 = "space")) || ((data.card === undefined) && (missing0 = "card"))) || ((data.titles === undefined) && (missing0 = "titles"))) || ((data.descriptions === undefined) && (missing0 = "descriptions"))) || ((data.columns === undefined) && (missing0 = "columns"))) || ((data.positions === undefined) && (missing0 = "positions"))) || ((data.completed === undefined) && (missing0 = "completed"))) || ((data.tags === undefined) && (missing0 = "tags"))) || ((data.state === undefined) && (missing0 = "state"))) || ((data.deletion_groups === undefined) && (missing0 = "deletion_groups"))) || ((data.version === undefined) && (missing0 = "version"))){
validate46.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(func1.call(schema61.properties, key0))){
validate46.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.card !== undefined){
const _errs2 = errors;
if(typeof data.card !== "string"){
validate46.errors = [{instancePath:instancePath+"/card",schemaPath:"#/properties/card/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid0 = _errs2 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.columns !== undefined){
let data1 = data.columns;
const _errs4 = errors;
if(errors === _errs4){
if(Array.isArray(data1)){
var valid1 = true;
const len0 = data1.length;
for(let i0=0; i0<len0; i0++){
const _errs6 = errors;
if(typeof data1[i0] !== "string"){
validate46.errors = [{instancePath:instancePath+"/columns/" + i0,schemaPath:"#/properties/columns/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid1 = _errs6 === errors;
if(!valid1){
break;
}
}
}
else {
validate46.errors = [{instancePath:instancePath+"/columns",schemaPath:"#/properties/columns/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid0 = _errs4 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.completed !== undefined){
let data3 = data.completed;
const _errs8 = errors;
if(errors === _errs8){
if(Array.isArray(data3)){
var valid2 = true;
const len1 = data3.length;
for(let i1=0; i1<len1; i1++){
const _errs10 = errors;
if(typeof data3[i1] !== "boolean"){
validate46.errors = [{instancePath:instancePath+"/completed/" + i1,schemaPath:"#/properties/completed/items/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid2 = _errs10 === errors;
if(!valid2){
break;
}
}
}
else {
validate46.errors = [{instancePath:instancePath+"/completed",schemaPath:"#/properties/completed/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid0 = _errs8 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.deletion_groups !== undefined){
let data5 = data.deletion_groups;
const _errs12 = errors;
if(errors === _errs12){
if(Array.isArray(data5)){
var valid3 = true;
const len2 = data5.length;
for(let i2=0; i2<len2; i2++){
const _errs14 = errors;
if(typeof data5[i2] !== "string"){
validate46.errors = [{instancePath:instancePath+"/deletion_groups/" + i2,schemaPath:"#/properties/deletion_groups/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid3 = _errs14 === errors;
if(!valid3){
break;
}
}
}
else {
validate46.errors = [{instancePath:instancePath+"/deletion_groups",schemaPath:"#/properties/deletion_groups/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid0 = _errs12 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.descriptions !== undefined){
let data7 = data.descriptions;
const _errs16 = errors;
if(errors === _errs16){
if(Array.isArray(data7)){
var valid4 = true;
const len3 = data7.length;
for(let i3=0; i3<len3; i3++){
const _errs18 = errors;
if(typeof data7[i3] !== "string"){
validate46.errors = [{instancePath:instancePath+"/descriptions/" + i3,schemaPath:"#/properties/descriptions/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid4 = _errs18 === errors;
if(!valid4){
break;
}
}
}
else {
validate46.errors = [{instancePath:instancePath+"/descriptions",schemaPath:"#/properties/descriptions/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid0 = _errs16 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.positions !== undefined){
let data9 = data.positions;
const _errs20 = errors;
if(errors === _errs20){
if(Array.isArray(data9)){
var valid5 = true;
const len4 = data9.length;
for(let i4=0; i4<len4; i4++){
let data10 = data9[i4];
const _errs22 = errors;
if(!((typeof data10 == "number") && (!(data10 % 1) && !isNaN(data10)))){
validate46.errors = [{instancePath:instancePath+"/positions/" + i4,schemaPath:"#/properties/positions/items/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs22){
if(typeof data10 == "number"){
if(data10 < 0 || isNaN(data10)){
validate46.errors = [{instancePath:instancePath+"/positions/" + i4,schemaPath:"#/properties/positions/items/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
var valid5 = _errs22 === errors;
if(!valid5){
break;
}
}
}
else {
validate46.errors = [{instancePath:instancePath+"/positions",schemaPath:"#/properties/positions/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid0 = _errs20 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.space !== undefined){
const _errs24 = errors;
if(typeof data.space !== "string"){
validate46.errors = [{instancePath:instancePath+"/space",schemaPath:"#/properties/space/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid0 = _errs24 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.state !== undefined){
let data12 = data.state;
const _errs26 = errors;
if(typeof data12 !== "string"){
validate46.errors = [{instancePath:instancePath+"/state",schemaPath:"#/$defs/SpaceState/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(((data12 === "active") || (data12 === "trashed")) || (data12 === "conflicted"))){
validate46.errors = [{instancePath:instancePath+"/state",schemaPath:"#/$defs/SpaceState/enum",keyword:"enum",params:{allowedValues: schema51.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs26 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.tags !== undefined){
let data13 = data.tags;
const _errs29 = errors;
if(errors === _errs29){
if(Array.isArray(data13)){
var valid7 = true;
const len5 = data13.length;
for(let i5=0; i5<len5; i5++){
const _errs31 = errors;
if(typeof data13[i5] !== "string"){
validate46.errors = [{instancePath:instancePath+"/tags/" + i5,schemaPath:"#/properties/tags/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid7 = _errs31 === errors;
if(!valid7){
break;
}
}
}
else {
validate46.errors = [{instancePath:instancePath+"/tags",schemaPath:"#/properties/tags/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid0 = _errs29 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.titles !== undefined){
let data15 = data.titles;
const _errs33 = errors;
if(errors === _errs33){
if(Array.isArray(data15)){
var valid8 = true;
const len6 = data15.length;
for(let i6=0; i6<len6; i6++){
const _errs35 = errors;
if(typeof data15[i6] !== "string"){
validate46.errors = [{instancePath:instancePath+"/titles/" + i6,schemaPath:"#/properties/titles/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid8 = _errs35 === errors;
if(!valid8){
break;
}
}
}
else {
validate46.errors = [{instancePath:instancePath+"/titles",schemaPath:"#/properties/titles/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid0 = _errs33 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.version !== undefined){
let data17 = data.version;
const _errs37 = errors;
const _errs38 = errors;
if(errors === _errs38){
if(Array.isArray(data17)){
if(data17.length > 32){
validate46.errors = [{instancePath:instancePath+"/version",schemaPath:"#/$defs/StateVersion/maxItems",keyword:"maxItems",params:{limit: 32},message:"must NOT have more than 32 items"}];
return false;
}
else {
if(data17.length < 32){
validate46.errors = [{instancePath:instancePath+"/version",schemaPath:"#/$defs/StateVersion/minItems",keyword:"minItems",params:{limit: 32},message:"must NOT have fewer than 32 items"}];
return false;
}
else {
var valid10 = true;
const len7 = data17.length;
for(let i7=0; i7<len7; i7++){
let data18 = data17[i7];
const _errs40 = errors;
if(!((typeof data18 == "number") && (!(data18 % 1) && !isNaN(data18)))){
validate46.errors = [{instancePath:instancePath+"/version/" + i7,schemaPath:"#/$defs/StateVersion/items/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs40){
if(typeof data18 == "number"){
if(data18 > 255 || isNaN(data18)){
validate46.errors = [{instancePath:instancePath+"/version/" + i7,schemaPath:"#/$defs/StateVersion/items/maximum",keyword:"maximum",params:{comparison: "<=", limit: 255},message:"must be <= 255"}];
return false;
}
else {
if(data18 < 0 || isNaN(data18)){
validate46.errors = [{instancePath:instancePath+"/version/" + i7,schemaPath:"#/$defs/StateVersion/items/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
}
var valid10 = _errs40 === errors;
if(!valid10){
break;
}
}
}
}
}
else {
validate46.errors = [{instancePath:instancePath+"/version",schemaPath:"#/$defs/StateVersion/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid0 = _errs37 === errors;
}
else {
var valid0 = true;
}
}
}
}
}
}
}
}
}
}
}
}
}
}
else {
validate46.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate46.errors = vErrors;
return errors === 0;
}
validate46.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate45(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate45.evaluated;
if(evaluated0.dynamicProps){
evaluated0.props = undefined;
}
if(evaluated0.dynamicItems){
evaluated0.items = undefined;
}
if(errors === 0){
if(data && typeof data == "object" && !Array.isArray(data)){
let missing0;
if((((data.status === undefined) && (missing0 = "status")) || ((data.result === undefined) && (missing0 = "result"))) || ((data.replayed === undefined) && (missing0 = "replayed"))){
validate45.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((key0 === "replayed") || (key0 === "result")) || (key0 === "status"))){
validate45.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.replayed !== undefined){
const _errs2 = errors;
if(typeof data.replayed !== "boolean"){
validate45.errors = [{instancePath:instancePath+"/replayed",schemaPath:"#/properties/replayed/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid0 = _errs2 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.result !== undefined){
let data1 = data.result;
const _errs4 = errors;
if(errors === _errs4){
if(data1 && typeof data1 == "object" && !Array.isArray(data1)){
let missing1;
if((data1.cards === undefined) && (missing1 = "cards")){
validate45.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/required",keyword:"required",params:{missingProperty: missing1},message:"must have required property '"+missing1+"'"}];
return false;
}
else {
const _errs6 = errors;
for(const key1 in data1){
if(!(key1 === "cards")){
validate45.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key1},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs6 === errors){
if(data1.cards !== undefined){
let data2 = data1.cards;
const _errs7 = errors;
if(errors === _errs7){
if(Array.isArray(data2)){
var valid2 = true;
const len0 = data2.length;
for(let i0=0; i0<len0; i0++){
const _errs9 = errors;
if(!(validate46(data2[i0], {instancePath:instancePath+"/result/cards/" + i0,parentData:data2,parentDataProperty:i0,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate46.errors : vErrors.concat(validate46.errors);
errors = vErrors.length;
}
var valid2 = _errs9 === errors;
if(!valid2){
break;
}
}
}
else {
validate45.errors = [{instancePath:instancePath+"/result/cards",schemaPath:"#/properties/result/properties/cards/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
}
}
}
}
else {
validate45.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
var valid0 = _errs4 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.status !== undefined){
let data4 = data.status;
const _errs10 = errors;
if(typeof data4 !== "string"){
validate45.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data4 === "ok")){
validate45.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/enum",keyword:"enum",params:{allowedValues: schema44.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs10 === errors;
}
else {
var valid0 = true;
}
}
}
}
}
}
else {
validate45.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate45.errors = vErrors;
return errors === 0;
}
validate45.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate44(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate44.evaluated;
if(evaluated0.dynamicProps){
evaluated0.props = undefined;
}
if(evaluated0.dynamicItems){
evaluated0.items = undefined;
}
const _errs0 = errors;
let valid0 = false;
let passing0 = null;
const _errs1 = errors;
if(!(validate45(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate45.errors : vErrors.concat(validate45.errors);
errors = vErrors.length;
}
var _valid0 = _errs1 === errors;
if(_valid0){
valid0 = true;
passing0 = 0;
var props0 = true;
}
const _errs2 = errors;
if(!(validate32(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate32.errors : vErrors.concat(validate32.errors);
errors = vErrors.length;
}
var _valid0 = _errs2 === errors;
if(_valid0 && valid0){
valid0 = false;
passing0 = [passing0, 1];
}
else {
if(_valid0){
valid0 = true;
passing0 = 1;
if(props0 !== true){
props0 = true;
}
}
}
if(!valid0){
const err0 = {instancePath,schemaPath:"#/oneOf",keyword:"oneOf",params:{passingSchemas: passing0},message:"must match exactly one schema in oneOf"};
if(vErrors === null){
vErrors = [err0];
}
else {
vErrors.push(err0);
}
errors++;
validate44.errors = vErrors;
return false;
}
else {
errors = _errs0;
if(vErrors !== null){
if(_errs0){
vErrors.length = _errs0;
}
else {
vErrors = null;
}
}
}
validate44.errors = vErrors;
evaluated0.props = props0;
return errors === 0;
}
validate44.evaluated = {"dynamicProps":true,"dynamicItems":false};

export const CommandResponse_ColumnSnapshot = validate50;
const schema65 = {"oneOf":[{"$ref":"#/$defs/CommandSuccess_ColumnSnapshot"},{"$ref":"#/$defs/CommandFailure"}],"description":"Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.\nA success confirms the original completion, not the current object state.\nConstruct success only AFTER the storage transaction commits. Cancellation,\na lost response or uncertain commit must never be converted to success."};
const schema66 = {"type":"object","required":["status","result","replayed"],"properties":{"replayed":{"type":"boolean"},"result":{"type":"object","required":["space","column","names","state","deletion_groups","positions","empty","version"],"properties":{"column":{"type":"string"},"deletion_groups":{"type":"array","items":{"type":"string"}},"empty":{"type":"boolean","description":"Unknown/future contents block trash and restore until supported."},"names":{"type":"array","items":{"type":"string"}},"positions":{"type":"array","items":{"type":"integer","format":"int64","minimum":0},"description":"All distinct position registers; conflicting values are never overwritten."},"space":{"type":"string"},"state":{"$ref":"#/$defs/SpaceState"},"version":{"$ref":"#/$defs/StateVersion"}},"additionalProperties":false},"status":{"$ref":"#/$defs/SuccessStatus"}},"additionalProperties":false};

function validate51(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate51.evaluated;
if(evaluated0.dynamicProps){
evaluated0.props = undefined;
}
if(evaluated0.dynamicItems){
evaluated0.items = undefined;
}
if(errors === 0){
if(data && typeof data == "object" && !Array.isArray(data)){
let missing0;
if((((data.status === undefined) && (missing0 = "status")) || ((data.result === undefined) && (missing0 = "result"))) || ((data.replayed === undefined) && (missing0 = "replayed"))){
validate51.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((key0 === "replayed") || (key0 === "result")) || (key0 === "status"))){
validate51.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.replayed !== undefined){
const _errs2 = errors;
if(typeof data.replayed !== "boolean"){
validate51.errors = [{instancePath:instancePath+"/replayed",schemaPath:"#/properties/replayed/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid0 = _errs2 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.result !== undefined){
let data1 = data.result;
const _errs4 = errors;
if(errors === _errs4){
if(data1 && typeof data1 == "object" && !Array.isArray(data1)){
let missing1;
if(((((((((data1.space === undefined) && (missing1 = "space")) || ((data1.column === undefined) && (missing1 = "column"))) || ((data1.names === undefined) && (missing1 = "names"))) || ((data1.state === undefined) && (missing1 = "state"))) || ((data1.deletion_groups === undefined) && (missing1 = "deletion_groups"))) || ((data1.positions === undefined) && (missing1 = "positions"))) || ((data1.empty === undefined) && (missing1 = "empty"))) || ((data1.version === undefined) && (missing1 = "version"))){
validate51.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/required",keyword:"required",params:{missingProperty: missing1},message:"must have required property '"+missing1+"'"}];
return false;
}
else {
const _errs6 = errors;
for(const key1 in data1){
if(!((((((((key1 === "column") || (key1 === "deletion_groups")) || (key1 === "empty")) || (key1 === "names")) || (key1 === "positions")) || (key1 === "space")) || (key1 === "state")) || (key1 === "version"))){
validate51.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key1},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs6 === errors){
if(data1.column !== undefined){
const _errs7 = errors;
if(typeof data1.column !== "string"){
validate51.errors = [{instancePath:instancePath+"/result/column",schemaPath:"#/properties/result/properties/column/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid1 = _errs7 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.deletion_groups !== undefined){
let data3 = data1.deletion_groups;
const _errs9 = errors;
if(errors === _errs9){
if(Array.isArray(data3)){
var valid2 = true;
const len0 = data3.length;
for(let i0=0; i0<len0; i0++){
const _errs11 = errors;
if(typeof data3[i0] !== "string"){
validate51.errors = [{instancePath:instancePath+"/result/deletion_groups/" + i0,schemaPath:"#/properties/result/properties/deletion_groups/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid2 = _errs11 === errors;
if(!valid2){
break;
}
}
}
else {
validate51.errors = [{instancePath:instancePath+"/result/deletion_groups",schemaPath:"#/properties/result/properties/deletion_groups/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs9 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.empty !== undefined){
const _errs13 = errors;
if(typeof data1.empty !== "boolean"){
validate51.errors = [{instancePath:instancePath+"/result/empty",schemaPath:"#/properties/result/properties/empty/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid1 = _errs13 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.names !== undefined){
let data6 = data1.names;
const _errs15 = errors;
if(errors === _errs15){
if(Array.isArray(data6)){
var valid3 = true;
const len1 = data6.length;
for(let i1=0; i1<len1; i1++){
const _errs17 = errors;
if(typeof data6[i1] !== "string"){
validate51.errors = [{instancePath:instancePath+"/result/names/" + i1,schemaPath:"#/properties/result/properties/names/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid3 = _errs17 === errors;
if(!valid3){
break;
}
}
}
else {
validate51.errors = [{instancePath:instancePath+"/result/names",schemaPath:"#/properties/result/properties/names/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs15 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.positions !== undefined){
let data8 = data1.positions;
const _errs19 = errors;
if(errors === _errs19){
if(Array.isArray(data8)){
var valid4 = true;
const len2 = data8.length;
for(let i2=0; i2<len2; i2++){
let data9 = data8[i2];
const _errs21 = errors;
if(!((typeof data9 == "number") && (!(data9 % 1) && !isNaN(data9)))){
validate51.errors = [{instancePath:instancePath+"/result/positions/" + i2,schemaPath:"#/properties/result/properties/positions/items/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs21){
if(typeof data9 == "number"){
if(data9 < 0 || isNaN(data9)){
validate51.errors = [{instancePath:instancePath+"/result/positions/" + i2,schemaPath:"#/properties/result/properties/positions/items/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
var valid4 = _errs21 === errors;
if(!valid4){
break;
}
}
}
else {
validate51.errors = [{instancePath:instancePath+"/result/positions",schemaPath:"#/properties/result/properties/positions/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs19 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.space !== undefined){
const _errs23 = errors;
if(typeof data1.space !== "string"){
validate51.errors = [{instancePath:instancePath+"/result/space",schemaPath:"#/properties/result/properties/space/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid1 = _errs23 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.state !== undefined){
let data11 = data1.state;
const _errs25 = errors;
if(typeof data11 !== "string"){
validate51.errors = [{instancePath:instancePath+"/result/state",schemaPath:"#/$defs/SpaceState/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(((data11 === "active") || (data11 === "trashed")) || (data11 === "conflicted"))){
validate51.errors = [{instancePath:instancePath+"/result/state",schemaPath:"#/$defs/SpaceState/enum",keyword:"enum",params:{allowedValues: schema51.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid1 = _errs25 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.version !== undefined){
let data12 = data1.version;
const _errs28 = errors;
const _errs29 = errors;
if(errors === _errs29){
if(Array.isArray(data12)){
if(data12.length > 32){
validate51.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/maxItems",keyword:"maxItems",params:{limit: 32},message:"must NOT have more than 32 items"}];
return false;
}
else {
if(data12.length < 32){
validate51.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/minItems",keyword:"minItems",params:{limit: 32},message:"must NOT have fewer than 32 items"}];
return false;
}
else {
var valid7 = true;
const len3 = data12.length;
for(let i3=0; i3<len3; i3++){
let data13 = data12[i3];
const _errs31 = errors;
if(!((typeof data13 == "number") && (!(data13 % 1) && !isNaN(data13)))){
validate51.errors = [{instancePath:instancePath+"/result/version/" + i3,schemaPath:"#/$defs/StateVersion/items/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs31){
if(typeof data13 == "number"){
if(data13 > 255 || isNaN(data13)){
validate51.errors = [{instancePath:instancePath+"/result/version/" + i3,schemaPath:"#/$defs/StateVersion/items/maximum",keyword:"maximum",params:{comparison: "<=", limit: 255},message:"must be <= 255"}];
return false;
}
else {
if(data13 < 0 || isNaN(data13)){
validate51.errors = [{instancePath:instancePath+"/result/version/" + i3,schemaPath:"#/$defs/StateVersion/items/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
}
var valid7 = _errs31 === errors;
if(!valid7){
break;
}
}
}
}
}
else {
validate51.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs28 === errors;
}
else {
var valid1 = true;
}
}
}
}
}
}
}
}
}
}
}
else {
validate51.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
var valid0 = _errs4 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.status !== undefined){
let data14 = data.status;
const _errs33 = errors;
if(typeof data14 !== "string"){
validate51.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data14 === "ok")){
validate51.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/enum",keyword:"enum",params:{allowedValues: schema44.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs33 === errors;
}
else {
var valid0 = true;
}
}
}
}
}
}
else {
validate51.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate51.errors = vErrors;
return errors === 0;
}
validate51.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate50(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate50.evaluated;
if(evaluated0.dynamicProps){
evaluated0.props = undefined;
}
if(evaluated0.dynamicItems){
evaluated0.items = undefined;
}
const _errs0 = errors;
let valid0 = false;
let passing0 = null;
const _errs1 = errors;
if(!(validate51(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate51.errors : vErrors.concat(validate51.errors);
errors = vErrors.length;
}
var _valid0 = _errs1 === errors;
if(_valid0){
valid0 = true;
passing0 = 0;
var props0 = true;
}
const _errs2 = errors;
if(!(validate32(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate32.errors : vErrors.concat(validate32.errors);
errors = vErrors.length;
}
var _valid0 = _errs2 === errors;
if(_valid0 && valid0){
valid0 = false;
passing0 = [passing0, 1];
}
else {
if(_valid0){
valid0 = true;
passing0 = 1;
if(props0 !== true){
props0 = true;
}
}
}
if(!valid0){
const err0 = {instancePath,schemaPath:"#/oneOf",keyword:"oneOf",params:{passingSchemas: passing0},message:"must match exactly one schema in oneOf"};
if(vErrors === null){
vErrors = [err0];
}
else {
vErrors.push(err0);
}
errors++;
validate50.errors = vErrors;
return false;
}
else {
errors = _errs0;
if(vErrors !== null){
if(_errs0){
vErrors.length = _errs0;
}
else {
vErrors = null;
}
}
}
validate50.errors = vErrors;
evaluated0.props = props0;
return errors === 0;
}
validate50.evaluated = {"dynamicProps":true,"dynamicItems":false};

export const CommandResponse_ColumnList = validate54;
const schema70 = {"oneOf":[{"$ref":"#/$defs/CommandSuccess_ColumnList"},{"$ref":"#/$defs/CommandFailure"}],"description":"Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.\nA success confirms the original completion, not the current object state.\nConstruct success only AFTER the storage transaction commits. Cancellation,\na lost response or uncertain commit must never be converted to success."};
const schema71 = {"type":"object","required":["status","result","replayed"],"properties":{"replayed":{"type":"boolean"},"result":{"type":"object","required":["columns"],"properties":{"columns":{"type":"array","items":{"$ref":"#/$defs/ColumnSnapshot"}}},"additionalProperties":false},"status":{"$ref":"#/$defs/SuccessStatus"}},"additionalProperties":false};
const schema72 = {"type":"object","required":["space","column","names","state","deletion_groups","positions","empty","version"],"properties":{"column":{"type":"string"},"deletion_groups":{"type":"array","items":{"type":"string"}},"empty":{"type":"boolean","description":"Unknown/future contents block trash and restore until supported."},"names":{"type":"array","items":{"type":"string"}},"positions":{"type":"array","items":{"type":"integer","format":"int64","minimum":0},"description":"All distinct position registers; conflicting values are never overwritten."},"space":{"type":"string"},"state":{"$ref":"#/$defs/SpaceState"},"version":{"$ref":"#/$defs/StateVersion"}},"additionalProperties":false};

function validate56(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate56.evaluated;
if(evaluated0.dynamicProps){
evaluated0.props = undefined;
}
if(evaluated0.dynamicItems){
evaluated0.items = undefined;
}
if(errors === 0){
if(data && typeof data == "object" && !Array.isArray(data)){
let missing0;
if(((((((((data.space === undefined) && (missing0 = "space")) || ((data.column === undefined) && (missing0 = "column"))) || ((data.names === undefined) && (missing0 = "names"))) || ((data.state === undefined) && (missing0 = "state"))) || ((data.deletion_groups === undefined) && (missing0 = "deletion_groups"))) || ((data.positions === undefined) && (missing0 = "positions"))) || ((data.empty === undefined) && (missing0 = "empty"))) || ((data.version === undefined) && (missing0 = "version"))){
validate56.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!((((((((key0 === "column") || (key0 === "deletion_groups")) || (key0 === "empty")) || (key0 === "names")) || (key0 === "positions")) || (key0 === "space")) || (key0 === "state")) || (key0 === "version"))){
validate56.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.column !== undefined){
const _errs2 = errors;
if(typeof data.column !== "string"){
validate56.errors = [{instancePath:instancePath+"/column",schemaPath:"#/properties/column/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid0 = _errs2 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.deletion_groups !== undefined){
let data1 = data.deletion_groups;
const _errs4 = errors;
if(errors === _errs4){
if(Array.isArray(data1)){
var valid1 = true;
const len0 = data1.length;
for(let i0=0; i0<len0; i0++){
const _errs6 = errors;
if(typeof data1[i0] !== "string"){
validate56.errors = [{instancePath:instancePath+"/deletion_groups/" + i0,schemaPath:"#/properties/deletion_groups/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid1 = _errs6 === errors;
if(!valid1){
break;
}
}
}
else {
validate56.errors = [{instancePath:instancePath+"/deletion_groups",schemaPath:"#/properties/deletion_groups/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid0 = _errs4 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.empty !== undefined){
const _errs8 = errors;
if(typeof data.empty !== "boolean"){
validate56.errors = [{instancePath:instancePath+"/empty",schemaPath:"#/properties/empty/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid0 = _errs8 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.names !== undefined){
let data4 = data.names;
const _errs10 = errors;
if(errors === _errs10){
if(Array.isArray(data4)){
var valid2 = true;
const len1 = data4.length;
for(let i1=0; i1<len1; i1++){
const _errs12 = errors;
if(typeof data4[i1] !== "string"){
validate56.errors = [{instancePath:instancePath+"/names/" + i1,schemaPath:"#/properties/names/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid2 = _errs12 === errors;
if(!valid2){
break;
}
}
}
else {
validate56.errors = [{instancePath:instancePath+"/names",schemaPath:"#/properties/names/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid0 = _errs10 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.positions !== undefined){
let data6 = data.positions;
const _errs14 = errors;
if(errors === _errs14){
if(Array.isArray(data6)){
var valid3 = true;
const len2 = data6.length;
for(let i2=0; i2<len2; i2++){
let data7 = data6[i2];
const _errs16 = errors;
if(!((typeof data7 == "number") && (!(data7 % 1) && !isNaN(data7)))){
validate56.errors = [{instancePath:instancePath+"/positions/" + i2,schemaPath:"#/properties/positions/items/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs16){
if(typeof data7 == "number"){
if(data7 < 0 || isNaN(data7)){
validate56.errors = [{instancePath:instancePath+"/positions/" + i2,schemaPath:"#/properties/positions/items/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
var valid3 = _errs16 === errors;
if(!valid3){
break;
}
}
}
else {
validate56.errors = [{instancePath:instancePath+"/positions",schemaPath:"#/properties/positions/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid0 = _errs14 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.space !== undefined){
const _errs18 = errors;
if(typeof data.space !== "string"){
validate56.errors = [{instancePath:instancePath+"/space",schemaPath:"#/properties/space/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid0 = _errs18 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.state !== undefined){
let data9 = data.state;
const _errs20 = errors;
if(typeof data9 !== "string"){
validate56.errors = [{instancePath:instancePath+"/state",schemaPath:"#/$defs/SpaceState/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(((data9 === "active") || (data9 === "trashed")) || (data9 === "conflicted"))){
validate56.errors = [{instancePath:instancePath+"/state",schemaPath:"#/$defs/SpaceState/enum",keyword:"enum",params:{allowedValues: schema51.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs20 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.version !== undefined){
let data10 = data.version;
const _errs23 = errors;
const _errs24 = errors;
if(errors === _errs24){
if(Array.isArray(data10)){
if(data10.length > 32){
validate56.errors = [{instancePath:instancePath+"/version",schemaPath:"#/$defs/StateVersion/maxItems",keyword:"maxItems",params:{limit: 32},message:"must NOT have more than 32 items"}];
return false;
}
else {
if(data10.length < 32){
validate56.errors = [{instancePath:instancePath+"/version",schemaPath:"#/$defs/StateVersion/minItems",keyword:"minItems",params:{limit: 32},message:"must NOT have fewer than 32 items"}];
return false;
}
else {
var valid6 = true;
const len3 = data10.length;
for(let i3=0; i3<len3; i3++){
let data11 = data10[i3];
const _errs26 = errors;
if(!((typeof data11 == "number") && (!(data11 % 1) && !isNaN(data11)))){
validate56.errors = [{instancePath:instancePath+"/version/" + i3,schemaPath:"#/$defs/StateVersion/items/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs26){
if(typeof data11 == "number"){
if(data11 > 255 || isNaN(data11)){
validate56.errors = [{instancePath:instancePath+"/version/" + i3,schemaPath:"#/$defs/StateVersion/items/maximum",keyword:"maximum",params:{comparison: "<=", limit: 255},message:"must be <= 255"}];
return false;
}
else {
if(data11 < 0 || isNaN(data11)){
validate56.errors = [{instancePath:instancePath+"/version/" + i3,schemaPath:"#/$defs/StateVersion/items/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
}
var valid6 = _errs26 === errors;
if(!valid6){
break;
}
}
}
}
}
else {
validate56.errors = [{instancePath:instancePath+"/version",schemaPath:"#/$defs/StateVersion/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid0 = _errs23 === errors;
}
else {
var valid0 = true;
}
}
}
}
}
}
}
}
}
}
}
else {
validate56.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate56.errors = vErrors;
return errors === 0;
}
validate56.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate55(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate55.evaluated;
if(evaluated0.dynamicProps){
evaluated0.props = undefined;
}
if(evaluated0.dynamicItems){
evaluated0.items = undefined;
}
if(errors === 0){
if(data && typeof data == "object" && !Array.isArray(data)){
let missing0;
if((((data.status === undefined) && (missing0 = "status")) || ((data.result === undefined) && (missing0 = "result"))) || ((data.replayed === undefined) && (missing0 = "replayed"))){
validate55.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((key0 === "replayed") || (key0 === "result")) || (key0 === "status"))){
validate55.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.replayed !== undefined){
const _errs2 = errors;
if(typeof data.replayed !== "boolean"){
validate55.errors = [{instancePath:instancePath+"/replayed",schemaPath:"#/properties/replayed/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid0 = _errs2 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.result !== undefined){
let data1 = data.result;
const _errs4 = errors;
if(errors === _errs4){
if(data1 && typeof data1 == "object" && !Array.isArray(data1)){
let missing1;
if((data1.columns === undefined) && (missing1 = "columns")){
validate55.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/required",keyword:"required",params:{missingProperty: missing1},message:"must have required property '"+missing1+"'"}];
return false;
}
else {
const _errs6 = errors;
for(const key1 in data1){
if(!(key1 === "columns")){
validate55.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key1},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs6 === errors){
if(data1.columns !== undefined){
let data2 = data1.columns;
const _errs7 = errors;
if(errors === _errs7){
if(Array.isArray(data2)){
var valid2 = true;
const len0 = data2.length;
for(let i0=0; i0<len0; i0++){
const _errs9 = errors;
if(!(validate56(data2[i0], {instancePath:instancePath+"/result/columns/" + i0,parentData:data2,parentDataProperty:i0,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate56.errors : vErrors.concat(validate56.errors);
errors = vErrors.length;
}
var valid2 = _errs9 === errors;
if(!valid2){
break;
}
}
}
else {
validate55.errors = [{instancePath:instancePath+"/result/columns",schemaPath:"#/properties/result/properties/columns/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
}
}
}
}
else {
validate55.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
var valid0 = _errs4 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.status !== undefined){
let data4 = data.status;
const _errs10 = errors;
if(typeof data4 !== "string"){
validate55.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data4 === "ok")){
validate55.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/enum",keyword:"enum",params:{allowedValues: schema44.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs10 === errors;
}
else {
var valid0 = true;
}
}
}
}
}
}
else {
validate55.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate55.errors = vErrors;
return errors === 0;
}
validate55.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate54(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate54.evaluated;
if(evaluated0.dynamicProps){
evaluated0.props = undefined;
}
if(evaluated0.dynamicItems){
evaluated0.items = undefined;
}
const _errs0 = errors;
let valid0 = false;
let passing0 = null;
const _errs1 = errors;
if(!(validate55(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate55.errors : vErrors.concat(validate55.errors);
errors = vErrors.length;
}
var _valid0 = _errs1 === errors;
if(_valid0){
valid0 = true;
passing0 = 0;
var props0 = true;
}
const _errs2 = errors;
if(!(validate32(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate32.errors : vErrors.concat(validate32.errors);
errors = vErrors.length;
}
var _valid0 = _errs2 === errors;
if(_valid0 && valid0){
valid0 = false;
passing0 = [passing0, 1];
}
else {
if(_valid0){
valid0 = true;
passing0 = 1;
if(props0 !== true){
props0 = true;
}
}
}
if(!valid0){
const err0 = {instancePath,schemaPath:"#/oneOf",keyword:"oneOf",params:{passingSchemas: passing0},message:"must match exactly one schema in oneOf"};
if(vErrors === null){
vErrors = [err0];
}
else {
vErrors.push(err0);
}
errors++;
validate54.errors = vErrors;
return false;
}
else {
errors = _errs0;
if(vErrors !== null){
if(_errs0){
vErrors.length = _errs0;
}
else {
vErrors = null;
}
}
}
validate54.errors = vErrors;
evaluated0.props = props0;
return errors === 0;
}
validate54.evaluated = {"dynamicProps":true,"dynamicItems":false};

export const CommandResponse_SpaceSnapshot = validate60;
const schema76 = {"oneOf":[{"$ref":"#/$defs/CommandSuccess_SpaceSnapshot"},{"$ref":"#/$defs/CommandFailure"}],"description":"Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.\nA success confirms the original completion, not the current object state.\nConstruct success only AFTER the storage transaction commits. Cancellation,\na lost response or uncertain commit must never be converted to success."};
const schema77 = {"type":"object","required":["status","result","replayed"],"properties":{"replayed":{"type":"boolean"},"result":{"type":"object","required":["space","names","state","deletion_groups","empty","version"],"properties":{"deletion_groups":{"type":"array","items":{"type":"string"}},"empty":{"type":"boolean","description":"This slice only trashes/restores spaces without domain content."},"names":{"type":"array","items":{"type":"string"},"description":"All distinct current names; multiple names require conflict resolution."},"space":{"type":"string"},"state":{"$ref":"#/$defs/SpaceState"},"version":{"$ref":"#/$defs/StateVersion"}},"additionalProperties":false},"status":{"$ref":"#/$defs/SuccessStatus"}},"additionalProperties":false};

function validate61(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate61.evaluated;
if(evaluated0.dynamicProps){
evaluated0.props = undefined;
}
if(evaluated0.dynamicItems){
evaluated0.items = undefined;
}
if(errors === 0){
if(data && typeof data == "object" && !Array.isArray(data)){
let missing0;
if((((data.status === undefined) && (missing0 = "status")) || ((data.result === undefined) && (missing0 = "result"))) || ((data.replayed === undefined) && (missing0 = "replayed"))){
validate61.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((key0 === "replayed") || (key0 === "result")) || (key0 === "status"))){
validate61.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.replayed !== undefined){
const _errs2 = errors;
if(typeof data.replayed !== "boolean"){
validate61.errors = [{instancePath:instancePath+"/replayed",schemaPath:"#/properties/replayed/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid0 = _errs2 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.result !== undefined){
let data1 = data.result;
const _errs4 = errors;
if(errors === _errs4){
if(data1 && typeof data1 == "object" && !Array.isArray(data1)){
let missing1;
if(((((((data1.space === undefined) && (missing1 = "space")) || ((data1.names === undefined) && (missing1 = "names"))) || ((data1.state === undefined) && (missing1 = "state"))) || ((data1.deletion_groups === undefined) && (missing1 = "deletion_groups"))) || ((data1.empty === undefined) && (missing1 = "empty"))) || ((data1.version === undefined) && (missing1 = "version"))){
validate61.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/required",keyword:"required",params:{missingProperty: missing1},message:"must have required property '"+missing1+"'"}];
return false;
}
else {
const _errs6 = errors;
for(const key1 in data1){
if(!((((((key1 === "deletion_groups") || (key1 === "empty")) || (key1 === "names")) || (key1 === "space")) || (key1 === "state")) || (key1 === "version"))){
validate61.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key1},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs6 === errors){
if(data1.deletion_groups !== undefined){
let data2 = data1.deletion_groups;
const _errs7 = errors;
if(errors === _errs7){
if(Array.isArray(data2)){
var valid2 = true;
const len0 = data2.length;
for(let i0=0; i0<len0; i0++){
const _errs9 = errors;
if(typeof data2[i0] !== "string"){
validate61.errors = [{instancePath:instancePath+"/result/deletion_groups/" + i0,schemaPath:"#/properties/result/properties/deletion_groups/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid2 = _errs9 === errors;
if(!valid2){
break;
}
}
}
else {
validate61.errors = [{instancePath:instancePath+"/result/deletion_groups",schemaPath:"#/properties/result/properties/deletion_groups/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs7 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.empty !== undefined){
const _errs11 = errors;
if(typeof data1.empty !== "boolean"){
validate61.errors = [{instancePath:instancePath+"/result/empty",schemaPath:"#/properties/result/properties/empty/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid1 = _errs11 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.names !== undefined){
let data5 = data1.names;
const _errs13 = errors;
if(errors === _errs13){
if(Array.isArray(data5)){
var valid3 = true;
const len1 = data5.length;
for(let i1=0; i1<len1; i1++){
const _errs15 = errors;
if(typeof data5[i1] !== "string"){
validate61.errors = [{instancePath:instancePath+"/result/names/" + i1,schemaPath:"#/properties/result/properties/names/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid3 = _errs15 === errors;
if(!valid3){
break;
}
}
}
else {
validate61.errors = [{instancePath:instancePath+"/result/names",schemaPath:"#/properties/result/properties/names/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs13 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.space !== undefined){
const _errs17 = errors;
if(typeof data1.space !== "string"){
validate61.errors = [{instancePath:instancePath+"/result/space",schemaPath:"#/properties/result/properties/space/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid1 = _errs17 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.state !== undefined){
let data8 = data1.state;
const _errs19 = errors;
if(typeof data8 !== "string"){
validate61.errors = [{instancePath:instancePath+"/result/state",schemaPath:"#/$defs/SpaceState/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(((data8 === "active") || (data8 === "trashed")) || (data8 === "conflicted"))){
validate61.errors = [{instancePath:instancePath+"/result/state",schemaPath:"#/$defs/SpaceState/enum",keyword:"enum",params:{allowedValues: schema51.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid1 = _errs19 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.version !== undefined){
let data9 = data1.version;
const _errs22 = errors;
const _errs23 = errors;
if(errors === _errs23){
if(Array.isArray(data9)){
if(data9.length > 32){
validate61.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/maxItems",keyword:"maxItems",params:{limit: 32},message:"must NOT have more than 32 items"}];
return false;
}
else {
if(data9.length < 32){
validate61.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/minItems",keyword:"minItems",params:{limit: 32},message:"must NOT have fewer than 32 items"}];
return false;
}
else {
var valid6 = true;
const len2 = data9.length;
for(let i2=0; i2<len2; i2++){
let data10 = data9[i2];
const _errs25 = errors;
if(!((typeof data10 == "number") && (!(data10 % 1) && !isNaN(data10)))){
validate61.errors = [{instancePath:instancePath+"/result/version/" + i2,schemaPath:"#/$defs/StateVersion/items/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs25){
if(typeof data10 == "number"){
if(data10 > 255 || isNaN(data10)){
validate61.errors = [{instancePath:instancePath+"/result/version/" + i2,schemaPath:"#/$defs/StateVersion/items/maximum",keyword:"maximum",params:{comparison: "<=", limit: 255},message:"must be <= 255"}];
return false;
}
else {
if(data10 < 0 || isNaN(data10)){
validate61.errors = [{instancePath:instancePath+"/result/version/" + i2,schemaPath:"#/$defs/StateVersion/items/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
}
var valid6 = _errs25 === errors;
if(!valid6){
break;
}
}
}
}
}
else {
validate61.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs22 === errors;
}
else {
var valid1 = true;
}
}
}
}
}
}
}
}
}
else {
validate61.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
var valid0 = _errs4 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.status !== undefined){
let data11 = data.status;
const _errs27 = errors;
if(typeof data11 !== "string"){
validate61.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data11 === "ok")){
validate61.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/enum",keyword:"enum",params:{allowedValues: schema44.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs27 === errors;
}
else {
var valid0 = true;
}
}
}
}
}
}
else {
validate61.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate61.errors = vErrors;
return errors === 0;
}
validate61.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate60(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate60.evaluated;
if(evaluated0.dynamicProps){
evaluated0.props = undefined;
}
if(evaluated0.dynamicItems){
evaluated0.items = undefined;
}
const _errs0 = errors;
let valid0 = false;
let passing0 = null;
const _errs1 = errors;
if(!(validate61(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate61.errors : vErrors.concat(validate61.errors);
errors = vErrors.length;
}
var _valid0 = _errs1 === errors;
if(_valid0){
valid0 = true;
passing0 = 0;
var props0 = true;
}
const _errs2 = errors;
if(!(validate32(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate32.errors : vErrors.concat(validate32.errors);
errors = vErrors.length;
}
var _valid0 = _errs2 === errors;
if(_valid0 && valid0){
valid0 = false;
passing0 = [passing0, 1];
}
else {
if(_valid0){
valid0 = true;
passing0 = 1;
if(props0 !== true){
props0 = true;
}
}
}
if(!valid0){
const err0 = {instancePath,schemaPath:"#/oneOf",keyword:"oneOf",params:{passingSchemas: passing0},message:"must match exactly one schema in oneOf"};
if(vErrors === null){
vErrors = [err0];
}
else {
vErrors.push(err0);
}
errors++;
validate60.errors = vErrors;
return false;
}
else {
errors = _errs0;
if(vErrors !== null){
if(_errs0){
vErrors.length = _errs0;
}
else {
vErrors = null;
}
}
}
validate60.errors = vErrors;
evaluated0.props = props0;
return errors === 0;
}
validate60.evaluated = {"dynamicProps":true,"dynamicItems":false};

export const CommandResponse_SpaceList = validate64;
const schema81 = {"oneOf":[{"$ref":"#/$defs/CommandSuccess_SpaceList"},{"$ref":"#/$defs/CommandFailure"}],"description":"Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.\nA success confirms the original completion, not the current object state.\nConstruct success only AFTER the storage transaction commits. Cancellation,\na lost response or uncertain commit must never be converted to success."};
const schema82 = {"type":"object","required":["status","result","replayed"],"properties":{"replayed":{"type":"boolean"},"result":{"type":"object","required":["spaces"],"properties":{"spaces":{"type":"array","items":{"$ref":"#/$defs/SpaceSnapshot"}}},"additionalProperties":false},"status":{"$ref":"#/$defs/SuccessStatus"}},"additionalProperties":false};
const schema83 = {"type":"object","required":["space","names","state","deletion_groups","empty","version"],"properties":{"deletion_groups":{"type":"array","items":{"type":"string"}},"empty":{"type":"boolean","description":"This slice only trashes/restores spaces without domain content."},"names":{"type":"array","items":{"type":"string"},"description":"All distinct current names; multiple names require conflict resolution."},"space":{"type":"string"},"state":{"$ref":"#/$defs/SpaceState"},"version":{"$ref":"#/$defs/StateVersion"}},"additionalProperties":false};

function validate66(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate66.evaluated;
if(evaluated0.dynamicProps){
evaluated0.props = undefined;
}
if(evaluated0.dynamicItems){
evaluated0.items = undefined;
}
if(errors === 0){
if(data && typeof data == "object" && !Array.isArray(data)){
let missing0;
if(((((((data.space === undefined) && (missing0 = "space")) || ((data.names === undefined) && (missing0 = "names"))) || ((data.state === undefined) && (missing0 = "state"))) || ((data.deletion_groups === undefined) && (missing0 = "deletion_groups"))) || ((data.empty === undefined) && (missing0 = "empty"))) || ((data.version === undefined) && (missing0 = "version"))){
validate66.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!((((((key0 === "deletion_groups") || (key0 === "empty")) || (key0 === "names")) || (key0 === "space")) || (key0 === "state")) || (key0 === "version"))){
validate66.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.deletion_groups !== undefined){
let data0 = data.deletion_groups;
const _errs2 = errors;
if(errors === _errs2){
if(Array.isArray(data0)){
var valid1 = true;
const len0 = data0.length;
for(let i0=0; i0<len0; i0++){
const _errs4 = errors;
if(typeof data0[i0] !== "string"){
validate66.errors = [{instancePath:instancePath+"/deletion_groups/" + i0,schemaPath:"#/properties/deletion_groups/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid1 = _errs4 === errors;
if(!valid1){
break;
}
}
}
else {
validate66.errors = [{instancePath:instancePath+"/deletion_groups",schemaPath:"#/properties/deletion_groups/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid0 = _errs2 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.empty !== undefined){
const _errs6 = errors;
if(typeof data.empty !== "boolean"){
validate66.errors = [{instancePath:instancePath+"/empty",schemaPath:"#/properties/empty/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid0 = _errs6 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.names !== undefined){
let data3 = data.names;
const _errs8 = errors;
if(errors === _errs8){
if(Array.isArray(data3)){
var valid2 = true;
const len1 = data3.length;
for(let i1=0; i1<len1; i1++){
const _errs10 = errors;
if(typeof data3[i1] !== "string"){
validate66.errors = [{instancePath:instancePath+"/names/" + i1,schemaPath:"#/properties/names/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid2 = _errs10 === errors;
if(!valid2){
break;
}
}
}
else {
validate66.errors = [{instancePath:instancePath+"/names",schemaPath:"#/properties/names/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid0 = _errs8 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.space !== undefined){
const _errs12 = errors;
if(typeof data.space !== "string"){
validate66.errors = [{instancePath:instancePath+"/space",schemaPath:"#/properties/space/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid0 = _errs12 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.state !== undefined){
let data6 = data.state;
const _errs14 = errors;
if(typeof data6 !== "string"){
validate66.errors = [{instancePath:instancePath+"/state",schemaPath:"#/$defs/SpaceState/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(((data6 === "active") || (data6 === "trashed")) || (data6 === "conflicted"))){
validate66.errors = [{instancePath:instancePath+"/state",schemaPath:"#/$defs/SpaceState/enum",keyword:"enum",params:{allowedValues: schema51.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs14 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.version !== undefined){
let data7 = data.version;
const _errs17 = errors;
const _errs18 = errors;
if(errors === _errs18){
if(Array.isArray(data7)){
if(data7.length > 32){
validate66.errors = [{instancePath:instancePath+"/version",schemaPath:"#/$defs/StateVersion/maxItems",keyword:"maxItems",params:{limit: 32},message:"must NOT have more than 32 items"}];
return false;
}
else {
if(data7.length < 32){
validate66.errors = [{instancePath:instancePath+"/version",schemaPath:"#/$defs/StateVersion/minItems",keyword:"minItems",params:{limit: 32},message:"must NOT have fewer than 32 items"}];
return false;
}
else {
var valid5 = true;
const len2 = data7.length;
for(let i2=0; i2<len2; i2++){
let data8 = data7[i2];
const _errs20 = errors;
if(!((typeof data8 == "number") && (!(data8 % 1) && !isNaN(data8)))){
validate66.errors = [{instancePath:instancePath+"/version/" + i2,schemaPath:"#/$defs/StateVersion/items/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs20){
if(typeof data8 == "number"){
if(data8 > 255 || isNaN(data8)){
validate66.errors = [{instancePath:instancePath+"/version/" + i2,schemaPath:"#/$defs/StateVersion/items/maximum",keyword:"maximum",params:{comparison: "<=", limit: 255},message:"must be <= 255"}];
return false;
}
else {
if(data8 < 0 || isNaN(data8)){
validate66.errors = [{instancePath:instancePath+"/version/" + i2,schemaPath:"#/$defs/StateVersion/items/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
}
var valid5 = _errs20 === errors;
if(!valid5){
break;
}
}
}
}
}
else {
validate66.errors = [{instancePath:instancePath+"/version",schemaPath:"#/$defs/StateVersion/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid0 = _errs17 === errors;
}
else {
var valid0 = true;
}
}
}
}
}
}
}
}
}
else {
validate66.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate66.errors = vErrors;
return errors === 0;
}
validate66.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate65(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate65.evaluated;
if(evaluated0.dynamicProps){
evaluated0.props = undefined;
}
if(evaluated0.dynamicItems){
evaluated0.items = undefined;
}
if(errors === 0){
if(data && typeof data == "object" && !Array.isArray(data)){
let missing0;
if((((data.status === undefined) && (missing0 = "status")) || ((data.result === undefined) && (missing0 = "result"))) || ((data.replayed === undefined) && (missing0 = "replayed"))){
validate65.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((key0 === "replayed") || (key0 === "result")) || (key0 === "status"))){
validate65.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.replayed !== undefined){
const _errs2 = errors;
if(typeof data.replayed !== "boolean"){
validate65.errors = [{instancePath:instancePath+"/replayed",schemaPath:"#/properties/replayed/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid0 = _errs2 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.result !== undefined){
let data1 = data.result;
const _errs4 = errors;
if(errors === _errs4){
if(data1 && typeof data1 == "object" && !Array.isArray(data1)){
let missing1;
if((data1.spaces === undefined) && (missing1 = "spaces")){
validate65.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/required",keyword:"required",params:{missingProperty: missing1},message:"must have required property '"+missing1+"'"}];
return false;
}
else {
const _errs6 = errors;
for(const key1 in data1){
if(!(key1 === "spaces")){
validate65.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key1},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs6 === errors){
if(data1.spaces !== undefined){
let data2 = data1.spaces;
const _errs7 = errors;
if(errors === _errs7){
if(Array.isArray(data2)){
var valid2 = true;
const len0 = data2.length;
for(let i0=0; i0<len0; i0++){
const _errs9 = errors;
if(!(validate66(data2[i0], {instancePath:instancePath+"/result/spaces/" + i0,parentData:data2,parentDataProperty:i0,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate66.errors : vErrors.concat(validate66.errors);
errors = vErrors.length;
}
var valid2 = _errs9 === errors;
if(!valid2){
break;
}
}
}
else {
validate65.errors = [{instancePath:instancePath+"/result/spaces",schemaPath:"#/properties/result/properties/spaces/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
}
}
}
}
else {
validate65.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
var valid0 = _errs4 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.status !== undefined){
let data4 = data.status;
const _errs10 = errors;
if(typeof data4 !== "string"){
validate65.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data4 === "ok")){
validate65.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/enum",keyword:"enum",params:{allowedValues: schema44.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs10 === errors;
}
else {
var valid0 = true;
}
}
}
}
}
}
else {
validate65.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate65.errors = vErrors;
return errors === 0;
}
validate65.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate64(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate64.evaluated;
if(evaluated0.dynamicProps){
evaluated0.props = undefined;
}
if(evaluated0.dynamicItems){
evaluated0.items = undefined;
}
const _errs0 = errors;
let valid0 = false;
let passing0 = null;
const _errs1 = errors;
if(!(validate65(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate65.errors : vErrors.concat(validate65.errors);
errors = vErrors.length;
}
var _valid0 = _errs1 === errors;
if(_valid0){
valid0 = true;
passing0 = 0;
var props0 = true;
}
const _errs2 = errors;
if(!(validate32(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate32.errors : vErrors.concat(validate32.errors);
errors = vErrors.length;
}
var _valid0 = _errs2 === errors;
if(_valid0 && valid0){
valid0 = false;
passing0 = [passing0, 1];
}
else {
if(_valid0){
valid0 = true;
passing0 = 1;
if(props0 !== true){
props0 = true;
}
}
}
if(!valid0){
const err0 = {instancePath,schemaPath:"#/oneOf",keyword:"oneOf",params:{passingSchemas: passing0},message:"must match exactly one schema in oneOf"};
if(vErrors === null){
vErrors = [err0];
}
else {
vErrors.push(err0);
}
errors++;
validate64.errors = vErrors;
return false;
}
else {
errors = _errs0;
if(vErrors !== null){
if(_errs0){
vErrors.length = _errs0;
}
else {
vErrors = null;
}
}
}
validate64.errors = vErrors;
evaluated0.props = props0;
return errors === 0;
}
validate64.evaluated = {"dynamicProps":true,"dynamicItems":false};

const responseValidators = Object.freeze({
"access.inspect": CommandResponse_AccessSnapshot,
"access.revoke": CommandResponse_AccessSnapshot,
"card.create": CommandResponse_CardSnapshot,
"card.edit": CommandResponse_CardSnapshot,
"card.history": CommandResponse_CardHistory,
"card.list": CommandResponse_CardList,
"card.read": CommandResponse_CardSnapshot,
"card.restore": CommandResponse_CardSnapshot,
"card.trash": CommandResponse_CardSnapshot,
"column.create": CommandResponse_ColumnSnapshot,
"column.list": CommandResponse_ColumnList,
"column.read": CommandResponse_ColumnSnapshot,
"column.rename": CommandResponse_ColumnSnapshot,
"column.restore": CommandResponse_ColumnSnapshot,
"column.trash": CommandResponse_ColumnSnapshot,
"space.create": CommandResponse_SpaceSnapshot,
"space.list": CommandResponse_SpaceList,
"space.read": CommandResponse_SpaceSnapshot,
"space.rename": CommandResponse_SpaceSnapshot,
"space.restore": CommandResponse_SpaceSnapshot,
"space.trash": CommandResponse_SpaceSnapshot
});
export const operations = Object.freeze(Object.keys(responseValidators));
export default function validate(value, operation) {
  if (operation !== undefined) return Object.hasOwn(responseValidators, operation) && responseValidators[operation](value);
  return Object.values(responseValidators).some(check => check(value));
}
