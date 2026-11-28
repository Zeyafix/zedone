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
const schema47 = {"type":"string","description":"Shared machine-readable errors, without credentials, SQL or platform details.\n\nCodes describe semantics, not HTTP statuses or retry policy. A stale request\nrequires a fresh read and a new explicit action. An unknown outcome requires\nchecking/replaying the original command, never inventing a new command ID.","enum":["invalid_request","incompatible_sync","access_denied","not_found","stale_state","conflict","command_mismatch","unavailable","outcome_unknown"]};

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
if(!(((((((((data0 === "invalid_request") || (data0 === "incompatible_sync")) || (data0 === "access_denied")) || (data0 === "not_found")) || (data0 === "stale_state")) || (data0 === "conflict")) || (data0 === "command_mismatch")) || (data0 === "unavailable")) || (data0 === "outcome_unknown"))){
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

export const CommandResponse_AccessList = validate36;
const schema49 = {"oneOf":[{"$ref":"#/$defs/CommandSuccess_AccessList"},{"$ref":"#/$defs/CommandFailure"}],"description":"Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.\nA success confirms the original completion, not the current object state.\nConstruct success only AFTER the storage transaction commits. Cancellation,\na lost response or uncertain commit must never be converted to success."};
const schema50 = {"type":"object","required":["status","result","replayed"],"properties":{"replayed":{"type":"boolean"},"result":{"type":"object","required":["entries"],"properties":{"entries":{"type":"array","items":{"$ref":"#/$defs/AccessSnapshot"},"description":"At most 50 entries, including revoked/expired grants. No secret/verifier.","maxItems":50},"next":{"type":["string","null"]}},"additionalProperties":false},"status":{"$ref":"#/$defs/SuccessStatus"}},"additionalProperties":false};
const schema51 = {"type":"object","description":"No keys or verifiers leave storage. Version includes retired/current key\nidentities internally so a regrant cannot revive a stale revocation request.","required":["subject","scope","revoked","version"],"properties":{"expires_at_unix_ms":{"type":["integer","null"],"format":"int64"},"revoked":{"type":"boolean"},"scope":{"$ref":"#/$defs/AccessScope"},"subject":{"type":"string"},"version":{"$ref":"#/$defs/StateVersion"}},"additionalProperties":false};

function validate38(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate38.evaluated;
if(evaluated0.dynamicProps){
evaluated0.props = undefined;
}
if(evaluated0.dynamicItems){
evaluated0.items = undefined;
}
if(errors === 0){
if(data && typeof data == "object" && !Array.isArray(data)){
let missing0;
if(((((data.subject === undefined) && (missing0 = "subject")) || ((data.scope === undefined) && (missing0 = "scope"))) || ((data.revoked === undefined) && (missing0 = "revoked"))) || ((data.version === undefined) && (missing0 = "version"))){
validate38.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((((key0 === "expires_at_unix_ms") || (key0 === "revoked")) || (key0 === "scope")) || (key0 === "subject")) || (key0 === "version"))){
validate38.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.expires_at_unix_ms !== undefined){
let data0 = data.expires_at_unix_ms;
const _errs2 = errors;
if((!((typeof data0 == "number") && (!(data0 % 1) && !isNaN(data0)))) && (data0 !== null)){
validate38.errors = [{instancePath:instancePath+"/expires_at_unix_ms",schemaPath:"#/properties/expires_at_unix_ms/type",keyword:"type",params:{type: schema51.properties.expires_at_unix_ms.type},message:"must be integer,null"}];
return false;
}
var valid0 = _errs2 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.revoked !== undefined){
const _errs4 = errors;
if(typeof data.revoked !== "boolean"){
validate38.errors = [{instancePath:instancePath+"/revoked",schemaPath:"#/properties/revoked/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid0 = _errs4 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.scope !== undefined){
const _errs6 = errors;
if(!(validate23(data.scope, {instancePath:instancePath+"/scope",parentData:data,parentDataProperty:"scope",rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate23.errors : vErrors.concat(validate23.errors);
errors = vErrors.length;
}
var valid0 = _errs6 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.subject !== undefined){
const _errs7 = errors;
if(typeof data.subject !== "string"){
validate38.errors = [{instancePath:instancePath+"/subject",schemaPath:"#/properties/subject/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid0 = _errs7 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.version !== undefined){
let data4 = data.version;
const _errs9 = errors;
const _errs10 = errors;
if(errors === _errs10){
if(Array.isArray(data4)){
if(data4.length > 32){
validate38.errors = [{instancePath:instancePath+"/version",schemaPath:"#/$defs/StateVersion/maxItems",keyword:"maxItems",params:{limit: 32},message:"must NOT have more than 32 items"}];
return false;
}
else {
if(data4.length < 32){
validate38.errors = [{instancePath:instancePath+"/version",schemaPath:"#/$defs/StateVersion/minItems",keyword:"minItems",params:{limit: 32},message:"must NOT have fewer than 32 items"}];
return false;
}
else {
var valid2 = true;
const len0 = data4.length;
for(let i0=0; i0<len0; i0++){
let data5 = data4[i0];
const _errs12 = errors;
if(!((typeof data5 == "number") && (!(data5 % 1) && !isNaN(data5)))){
validate38.errors = [{instancePath:instancePath+"/version/" + i0,schemaPath:"#/$defs/StateVersion/items/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs12){
if(typeof data5 == "number"){
if(data5 > 255 || isNaN(data5)){
validate38.errors = [{instancePath:instancePath+"/version/" + i0,schemaPath:"#/$defs/StateVersion/items/maximum",keyword:"maximum",params:{comparison: "<=", limit: 255},message:"must be <= 255"}];
return false;
}
else {
if(data5 < 0 || isNaN(data5)){
validate38.errors = [{instancePath:instancePath+"/version/" + i0,schemaPath:"#/$defs/StateVersion/items/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
}
var valid2 = _errs12 === errors;
if(!valid2){
break;
}
}
}
}
}
else {
validate38.errors = [{instancePath:instancePath+"/version",schemaPath:"#/$defs/StateVersion/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid0 = _errs9 === errors;
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
else {
validate38.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate38.errors = vErrors;
return errors === 0;
}
validate38.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


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
if((data1.entries === undefined) && (missing1 = "entries")){
validate37.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/required",keyword:"required",params:{missingProperty: missing1},message:"must have required property '"+missing1+"'"}];
return false;
}
else {
const _errs6 = errors;
for(const key1 in data1){
if(!((key1 === "entries") || (key1 === "next"))){
validate37.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key1},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs6 === errors){
if(data1.entries !== undefined){
let data2 = data1.entries;
const _errs7 = errors;
if(errors === _errs7){
if(Array.isArray(data2)){
if(data2.length > 50){
validate37.errors = [{instancePath:instancePath+"/result/entries",schemaPath:"#/properties/result/properties/entries/maxItems",keyword:"maxItems",params:{limit: 50},message:"must NOT have more than 50 items"}];
return false;
}
else {
var valid2 = true;
const len0 = data2.length;
for(let i0=0; i0<len0; i0++){
const _errs9 = errors;
if(!(validate38(data2[i0], {instancePath:instancePath+"/result/entries/" + i0,parentData:data2,parentDataProperty:i0,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate38.errors : vErrors.concat(validate38.errors);
errors = vErrors.length;
}
var valid2 = _errs9 === errors;
if(!valid2){
break;
}
}
}
}
else {
validate37.errors = [{instancePath:instancePath+"/result/entries",schemaPath:"#/properties/result/properties/entries/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs7 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.next !== undefined){
let data4 = data1.next;
const _errs10 = errors;
if((typeof data4 !== "string") && (data4 !== null)){
validate37.errors = [{instancePath:instancePath+"/result/next",schemaPath:"#/properties/result/properties/next/type",keyword:"type",params:{type: schema50.properties.result.properties.next.type},message:"must be string,null"}];
return false;
}
var valid1 = _errs10 === errors;
}
else {
var valid1 = true;
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
let data5 = data.status;
const _errs12 = errors;
if(typeof data5 !== "string"){
validate37.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data5 === "ok")){
validate37.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/enum",keyword:"enum",params:{allowedValues: schema44.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs12 === errors;
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

export const CommandResponse_BackupCatalog = validate43;
const schema54 = {"oneOf":[{"$ref":"#/$defs/CommandSuccess_BackupCatalog"},{"$ref":"#/$defs/CommandFailure"}],"description":"Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.\nA success confirms the original completion, not the current object state.\nConstruct success only AFTER the storage transaction commits. Cancellation,\na lost response or uncertain commit must never be converted to success."};
const schema55 = {"type":"object","required":["status","result","replayed"],"properties":{"replayed":{"type":"boolean"},"result":{"type":"object","required":["copies","unreadable"],"properties":{"copies":{"type":"array","items":{"$ref":"#/$defs/CatalogCopy"}},"unreadable":{"type":"integer","minimum":0}},"additionalProperties":false},"status":{"$ref":"#/$defs/SuccessStatus"}},"additionalProperties":false};
const schema56 = {"type":"object","required":["id","space","names","kind","created_at_unix_ms","digest"],"properties":{"created_at_unix_ms":{"type":"integer","format":"int64","minimum":0},"digest":{"$ref":"#/$defs/StateVersion"},"id":{"type":"string"},"kind":{"type":"string"},"names":{"type":"array","items":{"type":"string"}},"space":{"type":"string"}},"additionalProperties":false};

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
if(((((((data.id === undefined) && (missing0 = "id")) || ((data.space === undefined) && (missing0 = "space"))) || ((data.names === undefined) && (missing0 = "names"))) || ((data.kind === undefined) && (missing0 = "kind"))) || ((data.created_at_unix_ms === undefined) && (missing0 = "created_at_unix_ms"))) || ((data.digest === undefined) && (missing0 = "digest"))){
validate45.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!((((((key0 === "created_at_unix_ms") || (key0 === "digest")) || (key0 === "id")) || (key0 === "kind")) || (key0 === "names")) || (key0 === "space"))){
validate45.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.created_at_unix_ms !== undefined){
let data0 = data.created_at_unix_ms;
const _errs2 = errors;
if(!((typeof data0 == "number") && (!(data0 % 1) && !isNaN(data0)))){
validate45.errors = [{instancePath:instancePath+"/created_at_unix_ms",schemaPath:"#/properties/created_at_unix_ms/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs2){
if(typeof data0 == "number"){
if(data0 < 0 || isNaN(data0)){
validate45.errors = [{instancePath:instancePath+"/created_at_unix_ms",schemaPath:"#/properties/created_at_unix_ms/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
var valid0 = _errs2 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.digest !== undefined){
let data1 = data.digest;
const _errs4 = errors;
const _errs5 = errors;
if(errors === _errs5){
if(Array.isArray(data1)){
if(data1.length > 32){
validate45.errors = [{instancePath:instancePath+"/digest",schemaPath:"#/$defs/StateVersion/maxItems",keyword:"maxItems",params:{limit: 32},message:"must NOT have more than 32 items"}];
return false;
}
else {
if(data1.length < 32){
validate45.errors = [{instancePath:instancePath+"/digest",schemaPath:"#/$defs/StateVersion/minItems",keyword:"minItems",params:{limit: 32},message:"must NOT have fewer than 32 items"}];
return false;
}
else {
var valid2 = true;
const len0 = data1.length;
for(let i0=0; i0<len0; i0++){
let data2 = data1[i0];
const _errs7 = errors;
if(!((typeof data2 == "number") && (!(data2 % 1) && !isNaN(data2)))){
validate45.errors = [{instancePath:instancePath+"/digest/" + i0,schemaPath:"#/$defs/StateVersion/items/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs7){
if(typeof data2 == "number"){
if(data2 > 255 || isNaN(data2)){
validate45.errors = [{instancePath:instancePath+"/digest/" + i0,schemaPath:"#/$defs/StateVersion/items/maximum",keyword:"maximum",params:{comparison: "<=", limit: 255},message:"must be <= 255"}];
return false;
}
else {
if(data2 < 0 || isNaN(data2)){
validate45.errors = [{instancePath:instancePath+"/digest/" + i0,schemaPath:"#/$defs/StateVersion/items/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
}
var valid2 = _errs7 === errors;
if(!valid2){
break;
}
}
}
}
}
else {
validate45.errors = [{instancePath:instancePath+"/digest",schemaPath:"#/$defs/StateVersion/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid0 = _errs4 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.id !== undefined){
const _errs9 = errors;
if(typeof data.id !== "string"){
validate45.errors = [{instancePath:instancePath+"/id",schemaPath:"#/properties/id/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid0 = _errs9 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.kind !== undefined){
const _errs11 = errors;
if(typeof data.kind !== "string"){
validate45.errors = [{instancePath:instancePath+"/kind",schemaPath:"#/properties/kind/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid0 = _errs11 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.names !== undefined){
let data5 = data.names;
const _errs13 = errors;
if(errors === _errs13){
if(Array.isArray(data5)){
var valid3 = true;
const len1 = data5.length;
for(let i1=0; i1<len1; i1++){
const _errs15 = errors;
if(typeof data5[i1] !== "string"){
validate45.errors = [{instancePath:instancePath+"/names/" + i1,schemaPath:"#/properties/names/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid3 = _errs15 === errors;
if(!valid3){
break;
}
}
}
else {
validate45.errors = [{instancePath:instancePath+"/names",schemaPath:"#/properties/names/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid0 = _errs13 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.space !== undefined){
const _errs17 = errors;
if(typeof data.space !== "string"){
validate45.errors = [{instancePath:instancePath+"/space",schemaPath:"#/properties/space/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
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
if(errors === 0){
if(data && typeof data == "object" && !Array.isArray(data)){
let missing0;
if((((data.status === undefined) && (missing0 = "status")) || ((data.result === undefined) && (missing0 = "result"))) || ((data.replayed === undefined) && (missing0 = "replayed"))){
validate44.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((key0 === "replayed") || (key0 === "result")) || (key0 === "status"))){
validate44.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.replayed !== undefined){
const _errs2 = errors;
if(typeof data.replayed !== "boolean"){
validate44.errors = [{instancePath:instancePath+"/replayed",schemaPath:"#/properties/replayed/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
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
if(((data1.copies === undefined) && (missing1 = "copies")) || ((data1.unreadable === undefined) && (missing1 = "unreadable"))){
validate44.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/required",keyword:"required",params:{missingProperty: missing1},message:"must have required property '"+missing1+"'"}];
return false;
}
else {
const _errs6 = errors;
for(const key1 in data1){
if(!((key1 === "copies") || (key1 === "unreadable"))){
validate44.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key1},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs6 === errors){
if(data1.copies !== undefined){
let data2 = data1.copies;
const _errs7 = errors;
if(errors === _errs7){
if(Array.isArray(data2)){
var valid2 = true;
const len0 = data2.length;
for(let i0=0; i0<len0; i0++){
const _errs9 = errors;
if(!(validate45(data2[i0], {instancePath:instancePath+"/result/copies/" + i0,parentData:data2,parentDataProperty:i0,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate45.errors : vErrors.concat(validate45.errors);
errors = vErrors.length;
}
var valid2 = _errs9 === errors;
if(!valid2){
break;
}
}
}
else {
validate44.errors = [{instancePath:instancePath+"/result/copies",schemaPath:"#/properties/result/properties/copies/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs7 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.unreadable !== undefined){
let data4 = data1.unreadable;
const _errs10 = errors;
if(!((typeof data4 == "number") && (!(data4 % 1) && !isNaN(data4)))){
validate44.errors = [{instancePath:instancePath+"/result/unreadable",schemaPath:"#/properties/result/properties/unreadable/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs10){
if(typeof data4 == "number"){
if(data4 < 0 || isNaN(data4)){
validate44.errors = [{instancePath:instancePath+"/result/unreadable",schemaPath:"#/properties/result/properties/unreadable/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
var valid1 = _errs10 === errors;
}
else {
var valid1 = true;
}
}
}
}
}
else {
validate44.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/type",keyword:"type",params:{type: "object"},message:"must be object"}];
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
let data5 = data.status;
const _errs12 = errors;
if(typeof data5 !== "string"){
validate44.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data5 === "ok")){
validate44.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/enum",keyword:"enum",params:{allowedValues: schema44.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs12 === errors;
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
validate44.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate44.errors = vErrors;
return errors === 0;
}
validate44.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate43(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate43.evaluated;
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
if(!(validate44(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate44.errors : vErrors.concat(validate44.errors);
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
validate43.errors = vErrors;
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
validate43.errors = vErrors;
evaluated0.props = props0;
return errors === 0;
}
validate43.evaluated = {"dynamicProps":true,"dynamicItems":false};

export const CommandResponse_BackupDeleted = validate49;
const schema59 = {"oneOf":[{"$ref":"#/$defs/CommandSuccess_BackupDeleted"},{"$ref":"#/$defs/CommandFailure"}],"description":"Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.\nA success confirms the original completion, not the current object state.\nConstruct success only AFTER the storage transaction commits. Cancellation,\na lost response or uncertain commit must never be converted to success."};
const schema60 = {"type":"object","required":["status","result","replayed"],"properties":{"replayed":{"type":"boolean"},"result":{"type":"object","required":["deleted","cleanup_pending"],"properties":{"cleanup_pending":{"type":"boolean"},"deleted":{"type":"boolean"}},"additionalProperties":false},"status":{"$ref":"#/$defs/SuccessStatus"}},"additionalProperties":false};

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
if(errors === 0){
if(data && typeof data == "object" && !Array.isArray(data)){
let missing0;
if((((data.status === undefined) && (missing0 = "status")) || ((data.result === undefined) && (missing0 = "result"))) || ((data.replayed === undefined) && (missing0 = "replayed"))){
validate50.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((key0 === "replayed") || (key0 === "result")) || (key0 === "status"))){
validate50.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.replayed !== undefined){
const _errs2 = errors;
if(typeof data.replayed !== "boolean"){
validate50.errors = [{instancePath:instancePath+"/replayed",schemaPath:"#/properties/replayed/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
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
if(((data1.deleted === undefined) && (missing1 = "deleted")) || ((data1.cleanup_pending === undefined) && (missing1 = "cleanup_pending"))){
validate50.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/required",keyword:"required",params:{missingProperty: missing1},message:"must have required property '"+missing1+"'"}];
return false;
}
else {
const _errs6 = errors;
for(const key1 in data1){
if(!((key1 === "cleanup_pending") || (key1 === "deleted"))){
validate50.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key1},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs6 === errors){
if(data1.cleanup_pending !== undefined){
const _errs7 = errors;
if(typeof data1.cleanup_pending !== "boolean"){
validate50.errors = [{instancePath:instancePath+"/result/cleanup_pending",schemaPath:"#/properties/result/properties/cleanup_pending/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid1 = _errs7 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.deleted !== undefined){
const _errs9 = errors;
if(typeof data1.deleted !== "boolean"){
validate50.errors = [{instancePath:instancePath+"/result/deleted",schemaPath:"#/properties/result/properties/deleted/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid1 = _errs9 === errors;
}
else {
var valid1 = true;
}
}
}
}
}
else {
validate50.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/type",keyword:"type",params:{type: "object"},message:"must be object"}];
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
const _errs11 = errors;
if(typeof data4 !== "string"){
validate50.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data4 === "ok")){
validate50.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/enum",keyword:"enum",params:{allowedValues: schema44.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs11 === errors;
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
validate50.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate50.errors = vErrors;
return errors === 0;
}
validate50.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate49(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate49.evaluated;
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
if(!(validate50(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate50.errors : vErrors.concat(validate50.errors);
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
validate49.errors = vErrors;
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
validate49.errors = vErrors;
evaluated0.props = props0;
return errors === 0;
}
validate49.evaluated = {"dynamicProps":true,"dynamicItems":false};

export const CommandResponse_SpaceArchive = validate53;
const schema62 = {"oneOf":[{"$ref":"#/$defs/CommandSuccess_SpaceArchive"},{"$ref":"#/$defs/CommandFailure"}],"description":"Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.\nA success confirms the original completion, not the current object state.\nConstruct success only AFTER the storage transaction commits. Cancellation,\na lost response or uncertain commit must never be converted to success."};
const schema63 = {"type":"object","required":["status","result","replayed"],"properties":{"replayed":{"type":"boolean"},"result":{"type":"object","required":["archive"],"properties":{"archive":{"type":"string","pattern":"^([0-9a-f]{2}){1,1048576}(?![\\s\\S])"}},"additionalProperties":false},"status":{"$ref":"#/$defs/SuccessStatus"}},"additionalProperties":false};
const pattern4 = new RegExp("^([0-9a-f]{2}){1,1048576}(?![\\s\\S])", "u");

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
if(errors === 0){
if(data && typeof data == "object" && !Array.isArray(data)){
let missing0;
if((((data.status === undefined) && (missing0 = "status")) || ((data.result === undefined) && (missing0 = "result"))) || ((data.replayed === undefined) && (missing0 = "replayed"))){
validate54.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((key0 === "replayed") || (key0 === "result")) || (key0 === "status"))){
validate54.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.replayed !== undefined){
const _errs2 = errors;
if(typeof data.replayed !== "boolean"){
validate54.errors = [{instancePath:instancePath+"/replayed",schemaPath:"#/properties/replayed/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
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
if((data1.archive === undefined) && (missing1 = "archive")){
validate54.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/required",keyword:"required",params:{missingProperty: missing1},message:"must have required property '"+missing1+"'"}];
return false;
}
else {
const _errs6 = errors;
for(const key1 in data1){
if(!(key1 === "archive")){
validate54.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key1},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs6 === errors){
if(data1.archive !== undefined){
let data2 = data1.archive;
const _errs7 = errors;
if(errors === _errs7){
if(typeof data2 === "string"){
if(!pattern4.test(data2)){
validate54.errors = [{instancePath:instancePath+"/result/archive",schemaPath:"#/properties/result/properties/archive/pattern",keyword:"pattern",params:{pattern: "^([0-9a-f]{2}){1,1048576}(?![\\s\\S])"},message:"must match pattern \""+"^([0-9a-f]{2}){1,1048576}(?![\\s\\S])"+"\""}];
return false;
}
}
else {
validate54.errors = [{instancePath:instancePath+"/result/archive",schemaPath:"#/properties/result/properties/archive/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
}
}
}
}
}
else {
validate54.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/type",keyword:"type",params:{type: "object"},message:"must be object"}];
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
let data3 = data.status;
const _errs9 = errors;
if(typeof data3 !== "string"){
validate54.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data3 === "ok")){
validate54.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/enum",keyword:"enum",params:{allowedValues: schema44.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs9 === errors;
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
validate54.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate54.errors = vErrors;
return errors === 0;
}
validate54.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate53(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate53.evaluated;
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
if(!(validate54(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate54.errors : vErrors.concat(validate54.errors);
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
validate53.errors = vErrors;
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
validate53.errors = vErrors;
evaluated0.props = props0;
return errors === 0;
}
validate53.evaluated = {"dynamicProps":true,"dynamicItems":false};

export const CommandResponse_BackupFolders = validate57;
const schema65 = {"oneOf":[{"$ref":"#/$defs/CommandSuccess_BackupFolders"},{"$ref":"#/$defs/CommandFailure"}],"description":"Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.\nA success confirms the original completion, not the current object state.\nConstruct success only AFTER the storage transaction commits. Cancellation,\na lost response or uncertain commit must never be converted to success."};
const schema66 = {"type":"object","required":["status","result","replayed"],"properties":{"replayed":{"type":"boolean"},"result":{"type":"object","required":["folders"],"properties":{"folders":{"type":"array","items":{"$ref":"#/$defs/BackupFolder"}}},"additionalProperties":false},"status":{"$ref":"#/$defs/SuccessStatus"}},"additionalProperties":false};
const schema67 = {"type":"object","required":["id","label","writable"],"properties":{"id":{"type":"string"},"label":{"type":"string"},"writable":{"type":"boolean"}},"additionalProperties":false};

function validate58(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate58.evaluated;
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
validate58.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((key0 === "replayed") || (key0 === "result")) || (key0 === "status"))){
validate58.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.replayed !== undefined){
const _errs2 = errors;
if(typeof data.replayed !== "boolean"){
validate58.errors = [{instancePath:instancePath+"/replayed",schemaPath:"#/properties/replayed/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
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
if((data1.folders === undefined) && (missing1 = "folders")){
validate58.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/required",keyword:"required",params:{missingProperty: missing1},message:"must have required property '"+missing1+"'"}];
return false;
}
else {
const _errs6 = errors;
for(const key1 in data1){
if(!(key1 === "folders")){
validate58.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key1},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs6 === errors){
if(data1.folders !== undefined){
let data2 = data1.folders;
const _errs7 = errors;
if(errors === _errs7){
if(Array.isArray(data2)){
var valid2 = true;
const len0 = data2.length;
for(let i0=0; i0<len0; i0++){
let data3 = data2[i0];
const _errs9 = errors;
const _errs10 = errors;
if(errors === _errs10){
if(data3 && typeof data3 == "object" && !Array.isArray(data3)){
let missing2;
if((((data3.id === undefined) && (missing2 = "id")) || ((data3.label === undefined) && (missing2 = "label"))) || ((data3.writable === undefined) && (missing2 = "writable"))){
validate58.errors = [{instancePath:instancePath+"/result/folders/" + i0,schemaPath:"#/$defs/BackupFolder/required",keyword:"required",params:{missingProperty: missing2},message:"must have required property '"+missing2+"'"}];
return false;
}
else {
const _errs12 = errors;
for(const key2 in data3){
if(!(((key2 === "id") || (key2 === "label")) || (key2 === "writable"))){
validate58.errors = [{instancePath:instancePath+"/result/folders/" + i0,schemaPath:"#/$defs/BackupFolder/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key2},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs12 === errors){
if(data3.id !== undefined){
const _errs13 = errors;
if(typeof data3.id !== "string"){
validate58.errors = [{instancePath:instancePath+"/result/folders/" + i0+"/id",schemaPath:"#/$defs/BackupFolder/properties/id/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid4 = _errs13 === errors;
}
else {
var valid4 = true;
}
if(valid4){
if(data3.label !== undefined){
const _errs15 = errors;
if(typeof data3.label !== "string"){
validate58.errors = [{instancePath:instancePath+"/result/folders/" + i0+"/label",schemaPath:"#/$defs/BackupFolder/properties/label/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid4 = _errs15 === errors;
}
else {
var valid4 = true;
}
if(valid4){
if(data3.writable !== undefined){
const _errs17 = errors;
if(typeof data3.writable !== "boolean"){
validate58.errors = [{instancePath:instancePath+"/result/folders/" + i0+"/writable",schemaPath:"#/$defs/BackupFolder/properties/writable/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid4 = _errs17 === errors;
}
else {
var valid4 = true;
}
}
}
}
}
}
else {
validate58.errors = [{instancePath:instancePath+"/result/folders/" + i0,schemaPath:"#/$defs/BackupFolder/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
var valid2 = _errs9 === errors;
if(!valid2){
break;
}
}
}
else {
validate58.errors = [{instancePath:instancePath+"/result/folders",schemaPath:"#/properties/result/properties/folders/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
}
}
}
}
else {
validate58.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/type",keyword:"type",params:{type: "object"},message:"must be object"}];
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
let data7 = data.status;
const _errs19 = errors;
if(typeof data7 !== "string"){
validate58.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data7 === "ok")){
validate58.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/enum",keyword:"enum",params:{allowedValues: schema44.enum},message:"must be equal to one of the allowed values"}];
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
validate58.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate58.errors = vErrors;
return errors === 0;
}
validate58.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate57(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate57.evaluated;
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
if(!(validate58(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate58.errors : vErrors.concat(validate58.errors);
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
validate57.errors = vErrors;
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
validate57.errors = vErrors;
evaluated0.props = props0;
return errors === 0;
}
validate57.evaluated = {"dynamicProps":true,"dynamicItems":false};

export const CommandResponse_BackupLocation = validate61;
const schema69 = {"oneOf":[{"$ref":"#/$defs/CommandSuccess_BackupLocation"},{"$ref":"#/$defs/CommandFailure"}],"description":"Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.\nA success confirms the original completion, not the current object state.\nConstruct success only AFTER the storage transaction commits. Cancellation,\na lost response or uncertain commit must never be converted to success."};
const schema70 = {"type":"object","required":["status","result","replayed"],"properties":{"replayed":{"type":"boolean"},"result":{"type":"object","required":["folder","version","is_default","cleanup_pending"],"properties":{"cleanup_pending":{"type":"boolean"},"folder":{"type":"string"},"is_default":{"type":"boolean"},"version":{"$ref":"#/$defs/StateVersion"}},"additionalProperties":false},"status":{"$ref":"#/$defs/SuccessStatus"}},"additionalProperties":false};

function validate62(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate62.evaluated;
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
validate62.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((key0 === "replayed") || (key0 === "result")) || (key0 === "status"))){
validate62.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.replayed !== undefined){
const _errs2 = errors;
if(typeof data.replayed !== "boolean"){
validate62.errors = [{instancePath:instancePath+"/replayed",schemaPath:"#/properties/replayed/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
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
if(((((data1.folder === undefined) && (missing1 = "folder")) || ((data1.version === undefined) && (missing1 = "version"))) || ((data1.is_default === undefined) && (missing1 = "is_default"))) || ((data1.cleanup_pending === undefined) && (missing1 = "cleanup_pending"))){
validate62.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/required",keyword:"required",params:{missingProperty: missing1},message:"must have required property '"+missing1+"'"}];
return false;
}
else {
const _errs6 = errors;
for(const key1 in data1){
if(!((((key1 === "cleanup_pending") || (key1 === "folder")) || (key1 === "is_default")) || (key1 === "version"))){
validate62.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key1},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs6 === errors){
if(data1.cleanup_pending !== undefined){
const _errs7 = errors;
if(typeof data1.cleanup_pending !== "boolean"){
validate62.errors = [{instancePath:instancePath+"/result/cleanup_pending",schemaPath:"#/properties/result/properties/cleanup_pending/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid1 = _errs7 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.folder !== undefined){
const _errs9 = errors;
if(typeof data1.folder !== "string"){
validate62.errors = [{instancePath:instancePath+"/result/folder",schemaPath:"#/properties/result/properties/folder/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid1 = _errs9 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.is_default !== undefined){
const _errs11 = errors;
if(typeof data1.is_default !== "boolean"){
validate62.errors = [{instancePath:instancePath+"/result/is_default",schemaPath:"#/properties/result/properties/is_default/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
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
validate62.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/maxItems",keyword:"maxItems",params:{limit: 32},message:"must NOT have more than 32 items"}];
return false;
}
else {
if(data5.length < 32){
validate62.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/minItems",keyword:"minItems",params:{limit: 32},message:"must NOT have fewer than 32 items"}];
return false;
}
else {
var valid3 = true;
const len0 = data5.length;
for(let i0=0; i0<len0; i0++){
let data6 = data5[i0];
const _errs16 = errors;
if(!((typeof data6 == "number") && (!(data6 % 1) && !isNaN(data6)))){
validate62.errors = [{instancePath:instancePath+"/result/version/" + i0,schemaPath:"#/$defs/StateVersion/items/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs16){
if(typeof data6 == "number"){
if(data6 > 255 || isNaN(data6)){
validate62.errors = [{instancePath:instancePath+"/result/version/" + i0,schemaPath:"#/$defs/StateVersion/items/maximum",keyword:"maximum",params:{comparison: "<=", limit: 255},message:"must be <= 255"}];
return false;
}
else {
if(data6 < 0 || isNaN(data6)){
validate62.errors = [{instancePath:instancePath+"/result/version/" + i0,schemaPath:"#/$defs/StateVersion/items/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
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
validate62.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs13 === errors;
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
else {
validate62.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/type",keyword:"type",params:{type: "object"},message:"must be object"}];
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
let data7 = data.status;
const _errs18 = errors;
if(typeof data7 !== "string"){
validate62.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data7 === "ok")){
validate62.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/enum",keyword:"enum",params:{allowedValues: schema44.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs18 === errors;
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
validate62.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate62.errors = vErrors;
return errors === 0;
}
validate62.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


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
const _errs0 = errors;
let valid0 = false;
let passing0 = null;
const _errs1 = errors;
if(!(validate62(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate62.errors : vErrors.concat(validate62.errors);
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
validate61.errors = vErrors;
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
validate61.errors = vErrors;
evaluated0.props = props0;
return errors === 0;
}
validate61.evaluated = {"dynamicProps":true,"dynamicItems":false};

export const CommandResponse_CardSnapshot = validate65;
const schema73 = {"oneOf":[{"$ref":"#/$defs/CommandSuccess_CardSnapshot"},{"$ref":"#/$defs/CommandFailure"}],"description":"Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.\nA success confirms the original completion, not the current object state.\nConstruct success only AFTER the storage transaction commits. Cancellation,\na lost response or uncertain commit must never be converted to success."};
const schema74 = {"type":"object","required":["status","result","replayed"],"properties":{"replayed":{"type":"boolean"},"result":{"type":"object","required":["space","card","titles","descriptions","columns","positions","completed","tags","state","deletion_groups","version"],"properties":{"card":{"type":"string"},"columns":{"type":"array","items":{"type":"string"}},"completed":{"type":"array","items":{"type":"boolean"}},"deletion_conflict":{"type":"boolean"},"deletion_groups":{"type":"array","items":{"type":"string"}},"descriptions":{"type":"array","items":{"type":"string"}},"placements":{"type":"array","items":{"$ref":"#/$defs/CardPlacement"},"description":"Whole variants after a move; legacy documents and receipts omit these."},"positions":{"type":"array","items":{"$ref":"#/$defs/Rank"}},"space":{"type":"string"},"state":{"$ref":"#/$defs/SpaceState"},"tags":{"type":"array","items":{"type":"string"}},"titles":{"type":"array","items":{"type":"string"}},"version":{"$ref":"#/$defs/StateVersion"}},"additionalProperties":false},"status":{"$ref":"#/$defs/SuccessStatus"}},"additionalProperties":false};
const schema78 = {"type":"string","enum":["active","trashed","conflicted"]};
const func1 = Object.prototype.hasOwnProperty;
const schema75 = {"type":"object","description":"A whole register: concurrent moves cannot mix a column with another rank.","required":["column","rank"],"properties":{"column":{"type":"string"},"rank":{"$ref":"#/$defs/Rank"}},"additionalProperties":false};
const schema76 = {"oneOf":[{"type":"integer","format":"int64","minimum":0},{"type":"array","items":{"$ref":"#/$defs/RankPart"}}],"description":"Legacy integer positions remain valid in historical documents and receipts."};
const schema77 = {"type":"object","required":["digit","id"],"properties":{"digit":{"type":"integer","format":"int32","minimum":0},"id":{"type":"string"}},"additionalProperties":false};

function validate68(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate68.evaluated;
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
if(!((typeof data == "number") && (!(data % 1) && !isNaN(data)))){
const err0 = {instancePath,schemaPath:"#/oneOf/0/type",keyword:"type",params:{type: "integer"},message:"must be integer"};
if(vErrors === null){
vErrors = [err0];
}
else {
vErrors.push(err0);
}
errors++;
}
if(errors === _errs1){
if(typeof data == "number"){
if(data < 0 || isNaN(data)){
const err1 = {instancePath,schemaPath:"#/oneOf/0/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"};
if(vErrors === null){
vErrors = [err1];
}
else {
vErrors.push(err1);
}
errors++;
}
}
}
var _valid0 = _errs1 === errors;
if(_valid0){
valid0 = true;
passing0 = 0;
}
const _errs3 = errors;
if(errors === _errs3){
if(Array.isArray(data)){
var valid1 = true;
const len0 = data.length;
for(let i0=0; i0<len0; i0++){
let data0 = data[i0];
const _errs5 = errors;
const _errs6 = errors;
if(errors === _errs6){
if(data0 && typeof data0 == "object" && !Array.isArray(data0)){
let missing0;
if(((data0.digit === undefined) && (missing0 = "digit")) || ((data0.id === undefined) && (missing0 = "id"))){
const err2 = {instancePath:instancePath+"/" + i0,schemaPath:"#/$defs/RankPart/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"};
if(vErrors === null){
vErrors = [err2];
}
else {
vErrors.push(err2);
}
errors++;
}
else {
const _errs8 = errors;
for(const key0 in data0){
if(!((key0 === "digit") || (key0 === "id"))){
const err3 = {instancePath:instancePath+"/" + i0,schemaPath:"#/$defs/RankPart/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"};
if(vErrors === null){
vErrors = [err3];
}
else {
vErrors.push(err3);
}
errors++;
break;
}
}
if(_errs8 === errors){
if(data0.digit !== undefined){
let data1 = data0.digit;
const _errs9 = errors;
if(!((typeof data1 == "number") && (!(data1 % 1) && !isNaN(data1)))){
const err4 = {instancePath:instancePath+"/" + i0+"/digit",schemaPath:"#/$defs/RankPart/properties/digit/type",keyword:"type",params:{type: "integer"},message:"must be integer"};
if(vErrors === null){
vErrors = [err4];
}
else {
vErrors.push(err4);
}
errors++;
}
if(errors === _errs9){
if(typeof data1 == "number"){
if(data1 < 0 || isNaN(data1)){
const err5 = {instancePath:instancePath+"/" + i0+"/digit",schemaPath:"#/$defs/RankPart/properties/digit/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"};
if(vErrors === null){
vErrors = [err5];
}
else {
vErrors.push(err5);
}
errors++;
}
}
}
var valid3 = _errs9 === errors;
}
else {
var valid3 = true;
}
if(valid3){
if(data0.id !== undefined){
const _errs11 = errors;
if(typeof data0.id !== "string"){
const err6 = {instancePath:instancePath+"/" + i0+"/id",schemaPath:"#/$defs/RankPart/properties/id/type",keyword:"type",params:{type: "string"},message:"must be string"};
if(vErrors === null){
vErrors = [err6];
}
else {
vErrors.push(err6);
}
errors++;
}
var valid3 = _errs11 === errors;
}
else {
var valid3 = true;
}
}
}
}
}
else {
const err7 = {instancePath:instancePath+"/" + i0,schemaPath:"#/$defs/RankPart/type",keyword:"type",params:{type: "object"},message:"must be object"};
if(vErrors === null){
vErrors = [err7];
}
else {
vErrors.push(err7);
}
errors++;
}
}
var valid1 = _errs5 === errors;
if(!valid1){
break;
}
}
}
else {
const err8 = {instancePath,schemaPath:"#/oneOf/1/type",keyword:"type",params:{type: "array"},message:"must be array"};
if(vErrors === null){
vErrors = [err8];
}
else {
vErrors.push(err8);
}
errors++;
}
}
var _valid0 = _errs3 === errors;
if(_valid0 && valid0){
valid0 = false;
passing0 = [passing0, 1];
}
else {
if(_valid0){
valid0 = true;
passing0 = 1;
var items0 = true;
}
}
if(!valid0){
const err9 = {instancePath,schemaPath:"#/oneOf",keyword:"oneOf",params:{passingSchemas: passing0},message:"must match exactly one schema in oneOf"};
if(vErrors === null){
vErrors = [err9];
}
else {
vErrors.push(err9);
}
errors++;
validate68.errors = vErrors;
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
validate68.errors = vErrors;
evaluated0.items = items0;
return errors === 0;
}
validate68.evaluated = {"dynamicProps":false,"dynamicItems":true};


function validate67(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate67.evaluated;
if(evaluated0.dynamicProps){
evaluated0.props = undefined;
}
if(evaluated0.dynamicItems){
evaluated0.items = undefined;
}
if(errors === 0){
if(data && typeof data == "object" && !Array.isArray(data)){
let missing0;
if(((data.column === undefined) && (missing0 = "column")) || ((data.rank === undefined) && (missing0 = "rank"))){
validate67.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!((key0 === "column") || (key0 === "rank"))){
validate67.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.column !== undefined){
const _errs2 = errors;
if(typeof data.column !== "string"){
validate67.errors = [{instancePath:instancePath+"/column",schemaPath:"#/properties/column/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid0 = _errs2 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.rank !== undefined){
const _errs4 = errors;
if(!(validate68(data.rank, {instancePath:instancePath+"/rank",parentData:data,parentDataProperty:"rank",rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate68.errors : vErrors.concat(validate68.errors);
errors = vErrors.length;
}
var valid0 = _errs4 === errors;
}
else {
var valid0 = true;
}
}
}
}
}
else {
validate67.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate67.errors = vErrors;
return errors === 0;
}
validate67.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


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
if((((data.status === undefined) && (missing0 = "status")) || ((data.result === undefined) && (missing0 = "result"))) || ((data.replayed === undefined) && (missing0 = "replayed"))){
validate66.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((key0 === "replayed") || (key0 === "result")) || (key0 === "status"))){
validate66.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.replayed !== undefined){
const _errs2 = errors;
if(typeof data.replayed !== "boolean"){
validate66.errors = [{instancePath:instancePath+"/replayed",schemaPath:"#/properties/replayed/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
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
validate66.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/required",keyword:"required",params:{missingProperty: missing1},message:"must have required property '"+missing1+"'"}];
return false;
}
else {
const _errs6 = errors;
for(const key1 in data1){
if(!(func1.call(schema74.properties.result.properties, key1))){
validate66.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key1},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs6 === errors){
if(data1.card !== undefined){
const _errs7 = errors;
if(typeof data1.card !== "string"){
validate66.errors = [{instancePath:instancePath+"/result/card",schemaPath:"#/properties/result/properties/card/type",keyword:"type",params:{type: "string"},message:"must be string"}];
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
validate66.errors = [{instancePath:instancePath+"/result/columns/" + i0,schemaPath:"#/properties/result/properties/columns/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid2 = _errs11 === errors;
if(!valid2){
break;
}
}
}
else {
validate66.errors = [{instancePath:instancePath+"/result/columns",schemaPath:"#/properties/result/properties/columns/type",keyword:"type",params:{type: "array"},message:"must be array"}];
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
validate66.errors = [{instancePath:instancePath+"/result/completed/" + i1,schemaPath:"#/properties/result/properties/completed/items/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid3 = _errs15 === errors;
if(!valid3){
break;
}
}
}
else {
validate66.errors = [{instancePath:instancePath+"/result/completed",schemaPath:"#/properties/result/properties/completed/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs13 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.deletion_conflict !== undefined){
const _errs17 = errors;
if(typeof data1.deletion_conflict !== "boolean"){
validate66.errors = [{instancePath:instancePath+"/result/deletion_conflict",schemaPath:"#/properties/result/properties/deletion_conflict/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid1 = _errs17 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.deletion_groups !== undefined){
let data8 = data1.deletion_groups;
const _errs19 = errors;
if(errors === _errs19){
if(Array.isArray(data8)){
var valid4 = true;
const len2 = data8.length;
for(let i2=0; i2<len2; i2++){
const _errs21 = errors;
if(typeof data8[i2] !== "string"){
validate66.errors = [{instancePath:instancePath+"/result/deletion_groups/" + i2,schemaPath:"#/properties/result/properties/deletion_groups/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid4 = _errs21 === errors;
if(!valid4){
break;
}
}
}
else {
validate66.errors = [{instancePath:instancePath+"/result/deletion_groups",schemaPath:"#/properties/result/properties/deletion_groups/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs19 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.descriptions !== undefined){
let data10 = data1.descriptions;
const _errs23 = errors;
if(errors === _errs23){
if(Array.isArray(data10)){
var valid5 = true;
const len3 = data10.length;
for(let i3=0; i3<len3; i3++){
const _errs25 = errors;
if(typeof data10[i3] !== "string"){
validate66.errors = [{instancePath:instancePath+"/result/descriptions/" + i3,schemaPath:"#/properties/result/properties/descriptions/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid5 = _errs25 === errors;
if(!valid5){
break;
}
}
}
else {
validate66.errors = [{instancePath:instancePath+"/result/descriptions",schemaPath:"#/properties/result/properties/descriptions/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs23 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.placements !== undefined){
let data12 = data1.placements;
const _errs27 = errors;
if(errors === _errs27){
if(Array.isArray(data12)){
var valid6 = true;
const len4 = data12.length;
for(let i4=0; i4<len4; i4++){
const _errs29 = errors;
if(!(validate67(data12[i4], {instancePath:instancePath+"/result/placements/" + i4,parentData:data12,parentDataProperty:i4,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate67.errors : vErrors.concat(validate67.errors);
errors = vErrors.length;
}
var valid6 = _errs29 === errors;
if(!valid6){
break;
}
}
}
else {
validate66.errors = [{instancePath:instancePath+"/result/placements",schemaPath:"#/properties/result/properties/placements/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs27 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.positions !== undefined){
let data14 = data1.positions;
const _errs30 = errors;
if(errors === _errs30){
if(Array.isArray(data14)){
var valid7 = true;
const len5 = data14.length;
for(let i5=0; i5<len5; i5++){
const _errs32 = errors;
if(!(validate68(data14[i5], {instancePath:instancePath+"/result/positions/" + i5,parentData:data14,parentDataProperty:i5,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate68.errors : vErrors.concat(validate68.errors);
errors = vErrors.length;
}
var valid7 = _errs32 === errors;
if(!valid7){
break;
}
}
}
else {
validate66.errors = [{instancePath:instancePath+"/result/positions",schemaPath:"#/properties/result/properties/positions/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs30 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.space !== undefined){
const _errs33 = errors;
if(typeof data1.space !== "string"){
validate66.errors = [{instancePath:instancePath+"/result/space",schemaPath:"#/properties/result/properties/space/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid1 = _errs33 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.state !== undefined){
let data17 = data1.state;
const _errs35 = errors;
if(typeof data17 !== "string"){
validate66.errors = [{instancePath:instancePath+"/result/state",schemaPath:"#/$defs/SpaceState/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(((data17 === "active") || (data17 === "trashed")) || (data17 === "conflicted"))){
validate66.errors = [{instancePath:instancePath+"/result/state",schemaPath:"#/$defs/SpaceState/enum",keyword:"enum",params:{allowedValues: schema78.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid1 = _errs35 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.tags !== undefined){
let data18 = data1.tags;
const _errs38 = errors;
if(errors === _errs38){
if(Array.isArray(data18)){
var valid9 = true;
const len6 = data18.length;
for(let i6=0; i6<len6; i6++){
const _errs40 = errors;
if(typeof data18[i6] !== "string"){
validate66.errors = [{instancePath:instancePath+"/result/tags/" + i6,schemaPath:"#/properties/result/properties/tags/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid9 = _errs40 === errors;
if(!valid9){
break;
}
}
}
else {
validate66.errors = [{instancePath:instancePath+"/result/tags",schemaPath:"#/properties/result/properties/tags/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs38 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.titles !== undefined){
let data20 = data1.titles;
const _errs42 = errors;
if(errors === _errs42){
if(Array.isArray(data20)){
var valid10 = true;
const len7 = data20.length;
for(let i7=0; i7<len7; i7++){
const _errs44 = errors;
if(typeof data20[i7] !== "string"){
validate66.errors = [{instancePath:instancePath+"/result/titles/" + i7,schemaPath:"#/properties/result/properties/titles/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid10 = _errs44 === errors;
if(!valid10){
break;
}
}
}
else {
validate66.errors = [{instancePath:instancePath+"/result/titles",schemaPath:"#/properties/result/properties/titles/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs42 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.version !== undefined){
let data22 = data1.version;
const _errs46 = errors;
const _errs47 = errors;
if(errors === _errs47){
if(Array.isArray(data22)){
if(data22.length > 32){
validate66.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/maxItems",keyword:"maxItems",params:{limit: 32},message:"must NOT have more than 32 items"}];
return false;
}
else {
if(data22.length < 32){
validate66.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/minItems",keyword:"minItems",params:{limit: 32},message:"must NOT have fewer than 32 items"}];
return false;
}
else {
var valid12 = true;
const len8 = data22.length;
for(let i8=0; i8<len8; i8++){
let data23 = data22[i8];
const _errs49 = errors;
if(!((typeof data23 == "number") && (!(data23 % 1) && !isNaN(data23)))){
validate66.errors = [{instancePath:instancePath+"/result/version/" + i8,schemaPath:"#/$defs/StateVersion/items/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs49){
if(typeof data23 == "number"){
if(data23 > 255 || isNaN(data23)){
validate66.errors = [{instancePath:instancePath+"/result/version/" + i8,schemaPath:"#/$defs/StateVersion/items/maximum",keyword:"maximum",params:{comparison: "<=", limit: 255},message:"must be <= 255"}];
return false;
}
else {
if(data23 < 0 || isNaN(data23)){
validate66.errors = [{instancePath:instancePath+"/result/version/" + i8,schemaPath:"#/$defs/StateVersion/items/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
}
var valid12 = _errs49 === errors;
if(!valid12){
break;
}
}
}
}
}
else {
validate66.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs46 === errors;
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
}
}
else {
validate66.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/type",keyword:"type",params:{type: "object"},message:"must be object"}];
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
let data24 = data.status;
const _errs51 = errors;
if(typeof data24 !== "string"){
validate66.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data24 === "ok")){
validate66.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/enum",keyword:"enum",params:{allowedValues: schema44.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs51 === errors;
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
const _errs0 = errors;
let valid0 = false;
let passing0 = null;
const _errs1 = errors;
if(!(validate66(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate66.errors : vErrors.concat(validate66.errors);
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
validate65.errors = vErrors;
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
validate65.errors = vErrors;
evaluated0.props = props0;
return errors === 0;
}
validate65.evaluated = {"dynamicProps":true,"dynamicItems":false};

export const CommandResponse_CardHistory = validate74;
const schema81 = {"oneOf":[{"$ref":"#/$defs/CommandSuccess_CardHistory"},{"$ref":"#/$defs/CommandFailure"}],"description":"Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.\nA success confirms the original completion, not the current object state.\nConstruct success only AFTER the storage transaction commits. Cancellation,\na lost response or uncertain commit must never be converted to success."};
const schema82 = {"type":"object","required":["status","result","replayed"],"properties":{"replayed":{"type":"boolean"},"result":{"type":"object","required":["space","card","versions","version"],"properties":{"card":{"type":"string"},"next":{"type":["string","null"]},"space":{"type":"string"},"version":{"$ref":"#/$defs/StateVersion"},"versions":{"type":"array","items":{"$ref":"#/$defs/CardTextVersion"}}},"additionalProperties":false},"status":{"$ref":"#/$defs/SuccessStatus"}},"additionalProperties":false};
const schema84 = {"type":"object","required":["id","subject","sequence","title","description"],"properties":{"description":{"type":"string"},"id":{"type":"string"},"sequence":{"type":"integer","format":"int64","description":"Local causal sequence; independent branches may share a sequence.","minimum":0},"subject":{"type":"string"},"title":{"type":"string"}},"additionalProperties":false};

function validate75(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate75.evaluated;
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
validate75.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((key0 === "replayed") || (key0 === "result")) || (key0 === "status"))){
validate75.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.replayed !== undefined){
const _errs2 = errors;
if(typeof data.replayed !== "boolean"){
validate75.errors = [{instancePath:instancePath+"/replayed",schemaPath:"#/properties/replayed/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
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
validate75.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/required",keyword:"required",params:{missingProperty: missing1},message:"must have required property '"+missing1+"'"}];
return false;
}
else {
const _errs6 = errors;
for(const key1 in data1){
if(!(((((key1 === "card") || (key1 === "next")) || (key1 === "space")) || (key1 === "version")) || (key1 === "versions"))){
validate75.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key1},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs6 === errors){
if(data1.card !== undefined){
const _errs7 = errors;
if(typeof data1.card !== "string"){
validate75.errors = [{instancePath:instancePath+"/result/card",schemaPath:"#/properties/result/properties/card/type",keyword:"type",params:{type: "string"},message:"must be string"}];
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
validate75.errors = [{instancePath:instancePath+"/result/next",schemaPath:"#/properties/result/properties/next/type",keyword:"type",params:{type: schema82.properties.result.properties.next.type},message:"must be string,null"}];
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
validate75.errors = [{instancePath:instancePath+"/result/space",schemaPath:"#/properties/result/properties/space/type",keyword:"type",params:{type: "string"},message:"must be string"}];
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
validate75.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/maxItems",keyword:"maxItems",params:{limit: 32},message:"must NOT have more than 32 items"}];
return false;
}
else {
if(data5.length < 32){
validate75.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/minItems",keyword:"minItems",params:{limit: 32},message:"must NOT have fewer than 32 items"}];
return false;
}
else {
var valid3 = true;
const len0 = data5.length;
for(let i0=0; i0<len0; i0++){
let data6 = data5[i0];
const _errs16 = errors;
if(!((typeof data6 == "number") && (!(data6 % 1) && !isNaN(data6)))){
validate75.errors = [{instancePath:instancePath+"/result/version/" + i0,schemaPath:"#/$defs/StateVersion/items/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs16){
if(typeof data6 == "number"){
if(data6 > 255 || isNaN(data6)){
validate75.errors = [{instancePath:instancePath+"/result/version/" + i0,schemaPath:"#/$defs/StateVersion/items/maximum",keyword:"maximum",params:{comparison: "<=", limit: 255},message:"must be <= 255"}];
return false;
}
else {
if(data6 < 0 || isNaN(data6)){
validate75.errors = [{instancePath:instancePath+"/result/version/" + i0,schemaPath:"#/$defs/StateVersion/items/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
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
validate75.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/type",keyword:"type",params:{type: "array"},message:"must be array"}];
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
validate75.errors = [{instancePath:instancePath+"/result/versions/" + i1,schemaPath:"#/$defs/CardTextVersion/required",keyword:"required",params:{missingProperty: missing2},message:"must have required property '"+missing2+"'"}];
return false;
}
else {
const _errs23 = errors;
for(const key2 in data8){
if(!(((((key2 === "description") || (key2 === "id")) || (key2 === "sequence")) || (key2 === "subject")) || (key2 === "title"))){
validate75.errors = [{instancePath:instancePath+"/result/versions/" + i1,schemaPath:"#/$defs/CardTextVersion/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key2},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs23 === errors){
if(data8.description !== undefined){
const _errs24 = errors;
if(typeof data8.description !== "string"){
validate75.errors = [{instancePath:instancePath+"/result/versions/" + i1+"/description",schemaPath:"#/$defs/CardTextVersion/properties/description/type",keyword:"type",params:{type: "string"},message:"must be string"}];
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
validate75.errors = [{instancePath:instancePath+"/result/versions/" + i1+"/id",schemaPath:"#/$defs/CardTextVersion/properties/id/type",keyword:"type",params:{type: "string"},message:"must be string"}];
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
validate75.errors = [{instancePath:instancePath+"/result/versions/" + i1+"/sequence",schemaPath:"#/$defs/CardTextVersion/properties/sequence/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs28){
if(typeof data11 == "number"){
if(data11 < 0 || isNaN(data11)){
validate75.errors = [{instancePath:instancePath+"/result/versions/" + i1+"/sequence",schemaPath:"#/$defs/CardTextVersion/properties/sequence/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
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
validate75.errors = [{instancePath:instancePath+"/result/versions/" + i1+"/subject",schemaPath:"#/$defs/CardTextVersion/properties/subject/type",keyword:"type",params:{type: "string"},message:"must be string"}];
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
validate75.errors = [{instancePath:instancePath+"/result/versions/" + i1+"/title",schemaPath:"#/$defs/CardTextVersion/properties/title/type",keyword:"type",params:{type: "string"},message:"must be string"}];
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
validate75.errors = [{instancePath:instancePath+"/result/versions/" + i1,schemaPath:"#/$defs/CardTextVersion/type",keyword:"type",params:{type: "object"},message:"must be object"}];
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
validate75.errors = [{instancePath:instancePath+"/result/versions",schemaPath:"#/properties/result/properties/versions/type",keyword:"type",params:{type: "array"},message:"must be array"}];
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
validate75.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/type",keyword:"type",params:{type: "object"},message:"must be object"}];
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
validate75.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data14 === "ok")){
validate75.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/enum",keyword:"enum",params:{allowedValues: schema44.enum},message:"must be equal to one of the allowed values"}];
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
validate75.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate75.errors = vErrors;
return errors === 0;
}
validate75.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate74(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate74.evaluated;
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
if(!(validate75(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate75.errors : vErrors.concat(validate75.errors);
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
validate74.errors = vErrors;
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
validate74.errors = vErrors;
evaluated0.props = props0;
return errors === 0;
}
validate74.evaluated = {"dynamicProps":true,"dynamicItems":false};

export const CommandResponse_CardPlacements = validate78;
const schema86 = {"oneOf":[{"$ref":"#/$defs/CommandSuccess_CardPlacements"},{"$ref":"#/$defs/CommandFailure"}],"description":"Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.\nA success confirms the original completion, not the current object state.\nConstruct success only AFTER the storage transaction commits. Cancellation,\na lost response or uncertain commit must never be converted to success."};
const schema87 = {"type":"object","required":["status","result","replayed"],"properties":{"replayed":{"type":"boolean"},"result":{"type":"object","required":["card","choices"],"properties":{"card":{"$ref":"#/$defs/CardSnapshot"},"choices":{"type":"array","items":{"$ref":"#/$defs/PlacementChoice"}}},"additionalProperties":false},"status":{"$ref":"#/$defs/SuccessStatus"}},"additionalProperties":false};
const schema88 = {"type":"object","required":["space","card","titles","descriptions","columns","positions","completed","tags","state","deletion_groups","version"],"properties":{"card":{"type":"string"},"columns":{"type":"array","items":{"type":"string"}},"completed":{"type":"array","items":{"type":"boolean"}},"deletion_conflict":{"type":"boolean"},"deletion_groups":{"type":"array","items":{"type":"string"}},"descriptions":{"type":"array","items":{"type":"string"}},"placements":{"type":"array","items":{"$ref":"#/$defs/CardPlacement"},"description":"Whole variants after a move; legacy documents and receipts omit these."},"positions":{"type":"array","items":{"$ref":"#/$defs/Rank"}},"space":{"type":"string"},"state":{"$ref":"#/$defs/SpaceState"},"tags":{"type":"array","items":{"type":"string"}},"titles":{"type":"array","items":{"type":"string"}},"version":{"$ref":"#/$defs/StateVersion"}},"additionalProperties":false};

function validate80(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate80.evaluated;
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
validate80.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(func1.call(schema88.properties, key0))){
validate80.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.card !== undefined){
const _errs2 = errors;
if(typeof data.card !== "string"){
validate80.errors = [{instancePath:instancePath+"/card",schemaPath:"#/properties/card/type",keyword:"type",params:{type: "string"},message:"must be string"}];
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
validate80.errors = [{instancePath:instancePath+"/columns/" + i0,schemaPath:"#/properties/columns/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid1 = _errs6 === errors;
if(!valid1){
break;
}
}
}
else {
validate80.errors = [{instancePath:instancePath+"/columns",schemaPath:"#/properties/columns/type",keyword:"type",params:{type: "array"},message:"must be array"}];
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
validate80.errors = [{instancePath:instancePath+"/completed/" + i1,schemaPath:"#/properties/completed/items/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid2 = _errs10 === errors;
if(!valid2){
break;
}
}
}
else {
validate80.errors = [{instancePath:instancePath+"/completed",schemaPath:"#/properties/completed/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid0 = _errs8 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.deletion_conflict !== undefined){
const _errs12 = errors;
if(typeof data.deletion_conflict !== "boolean"){
validate80.errors = [{instancePath:instancePath+"/deletion_conflict",schemaPath:"#/properties/deletion_conflict/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid0 = _errs12 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.deletion_groups !== undefined){
let data6 = data.deletion_groups;
const _errs14 = errors;
if(errors === _errs14){
if(Array.isArray(data6)){
var valid3 = true;
const len2 = data6.length;
for(let i2=0; i2<len2; i2++){
const _errs16 = errors;
if(typeof data6[i2] !== "string"){
validate80.errors = [{instancePath:instancePath+"/deletion_groups/" + i2,schemaPath:"#/properties/deletion_groups/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid3 = _errs16 === errors;
if(!valid3){
break;
}
}
}
else {
validate80.errors = [{instancePath:instancePath+"/deletion_groups",schemaPath:"#/properties/deletion_groups/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid0 = _errs14 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.descriptions !== undefined){
let data8 = data.descriptions;
const _errs18 = errors;
if(errors === _errs18){
if(Array.isArray(data8)){
var valid4 = true;
const len3 = data8.length;
for(let i3=0; i3<len3; i3++){
const _errs20 = errors;
if(typeof data8[i3] !== "string"){
validate80.errors = [{instancePath:instancePath+"/descriptions/" + i3,schemaPath:"#/properties/descriptions/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid4 = _errs20 === errors;
if(!valid4){
break;
}
}
}
else {
validate80.errors = [{instancePath:instancePath+"/descriptions",schemaPath:"#/properties/descriptions/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid0 = _errs18 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.placements !== undefined){
let data10 = data.placements;
const _errs22 = errors;
if(errors === _errs22){
if(Array.isArray(data10)){
var valid5 = true;
const len4 = data10.length;
for(let i4=0; i4<len4; i4++){
const _errs24 = errors;
if(!(validate67(data10[i4], {instancePath:instancePath+"/placements/" + i4,parentData:data10,parentDataProperty:i4,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate67.errors : vErrors.concat(validate67.errors);
errors = vErrors.length;
}
var valid5 = _errs24 === errors;
if(!valid5){
break;
}
}
}
else {
validate80.errors = [{instancePath:instancePath+"/placements",schemaPath:"#/properties/placements/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid0 = _errs22 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.positions !== undefined){
let data12 = data.positions;
const _errs25 = errors;
if(errors === _errs25){
if(Array.isArray(data12)){
var valid6 = true;
const len5 = data12.length;
for(let i5=0; i5<len5; i5++){
const _errs27 = errors;
if(!(validate68(data12[i5], {instancePath:instancePath+"/positions/" + i5,parentData:data12,parentDataProperty:i5,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate68.errors : vErrors.concat(validate68.errors);
errors = vErrors.length;
}
var valid6 = _errs27 === errors;
if(!valid6){
break;
}
}
}
else {
validate80.errors = [{instancePath:instancePath+"/positions",schemaPath:"#/properties/positions/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid0 = _errs25 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.space !== undefined){
const _errs28 = errors;
if(typeof data.space !== "string"){
validate80.errors = [{instancePath:instancePath+"/space",schemaPath:"#/properties/space/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid0 = _errs28 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.state !== undefined){
let data15 = data.state;
const _errs30 = errors;
if(typeof data15 !== "string"){
validate80.errors = [{instancePath:instancePath+"/state",schemaPath:"#/$defs/SpaceState/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(((data15 === "active") || (data15 === "trashed")) || (data15 === "conflicted"))){
validate80.errors = [{instancePath:instancePath+"/state",schemaPath:"#/$defs/SpaceState/enum",keyword:"enum",params:{allowedValues: schema78.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs30 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.tags !== undefined){
let data16 = data.tags;
const _errs33 = errors;
if(errors === _errs33){
if(Array.isArray(data16)){
var valid8 = true;
const len6 = data16.length;
for(let i6=0; i6<len6; i6++){
const _errs35 = errors;
if(typeof data16[i6] !== "string"){
validate80.errors = [{instancePath:instancePath+"/tags/" + i6,schemaPath:"#/properties/tags/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid8 = _errs35 === errors;
if(!valid8){
break;
}
}
}
else {
validate80.errors = [{instancePath:instancePath+"/tags",schemaPath:"#/properties/tags/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid0 = _errs33 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.titles !== undefined){
let data18 = data.titles;
const _errs37 = errors;
if(errors === _errs37){
if(Array.isArray(data18)){
var valid9 = true;
const len7 = data18.length;
for(let i7=0; i7<len7; i7++){
const _errs39 = errors;
if(typeof data18[i7] !== "string"){
validate80.errors = [{instancePath:instancePath+"/titles/" + i7,schemaPath:"#/properties/titles/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid9 = _errs39 === errors;
if(!valid9){
break;
}
}
}
else {
validate80.errors = [{instancePath:instancePath+"/titles",schemaPath:"#/properties/titles/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid0 = _errs37 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.version !== undefined){
let data20 = data.version;
const _errs41 = errors;
const _errs42 = errors;
if(errors === _errs42){
if(Array.isArray(data20)){
if(data20.length > 32){
validate80.errors = [{instancePath:instancePath+"/version",schemaPath:"#/$defs/StateVersion/maxItems",keyword:"maxItems",params:{limit: 32},message:"must NOT have more than 32 items"}];
return false;
}
else {
if(data20.length < 32){
validate80.errors = [{instancePath:instancePath+"/version",schemaPath:"#/$defs/StateVersion/minItems",keyword:"minItems",params:{limit: 32},message:"must NOT have fewer than 32 items"}];
return false;
}
else {
var valid11 = true;
const len8 = data20.length;
for(let i8=0; i8<len8; i8++){
let data21 = data20[i8];
const _errs44 = errors;
if(!((typeof data21 == "number") && (!(data21 % 1) && !isNaN(data21)))){
validate80.errors = [{instancePath:instancePath+"/version/" + i8,schemaPath:"#/$defs/StateVersion/items/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs44){
if(typeof data21 == "number"){
if(data21 > 255 || isNaN(data21)){
validate80.errors = [{instancePath:instancePath+"/version/" + i8,schemaPath:"#/$defs/StateVersion/items/maximum",keyword:"maximum",params:{comparison: "<=", limit: 255},message:"must be <= 255"}];
return false;
}
else {
if(data21 < 0 || isNaN(data21)){
validate80.errors = [{instancePath:instancePath+"/version/" + i8,schemaPath:"#/$defs/StateVersion/items/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
}
var valid11 = _errs44 === errors;
if(!valid11){
break;
}
}
}
}
}
else {
validate80.errors = [{instancePath:instancePath+"/version",schemaPath:"#/$defs/StateVersion/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid0 = _errs41 === errors;
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
}
}
else {
validate80.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate80.errors = vErrors;
return errors === 0;
}
validate80.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};

const schema91 = {"type":"object","required":["placement"],"properties":{"before":{"type":["string","null"]},"placement":{"$ref":"#/$defs/CardPlacement"}},"additionalProperties":false};

function validate84(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate84.evaluated;
if(evaluated0.dynamicProps){
evaluated0.props = undefined;
}
if(evaluated0.dynamicItems){
evaluated0.items = undefined;
}
if(errors === 0){
if(data && typeof data == "object" && !Array.isArray(data)){
let missing0;
if((data.placement === undefined) && (missing0 = "placement")){
validate84.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!((key0 === "before") || (key0 === "placement"))){
validate84.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.before !== undefined){
let data0 = data.before;
const _errs2 = errors;
if((typeof data0 !== "string") && (data0 !== null)){
validate84.errors = [{instancePath:instancePath+"/before",schemaPath:"#/properties/before/type",keyword:"type",params:{type: schema91.properties.before.type},message:"must be string,null"}];
return false;
}
var valid0 = _errs2 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.placement !== undefined){
const _errs4 = errors;
if(!(validate67(data.placement, {instancePath:instancePath+"/placement",parentData:data,parentDataProperty:"placement",rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate67.errors : vErrors.concat(validate67.errors);
errors = vErrors.length;
}
var valid0 = _errs4 === errors;
}
else {
var valid0 = true;
}
}
}
}
}
else {
validate84.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate84.errors = vErrors;
return errors === 0;
}
validate84.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate79(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate79.evaluated;
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
validate79.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((key0 === "replayed") || (key0 === "result")) || (key0 === "status"))){
validate79.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.replayed !== undefined){
const _errs2 = errors;
if(typeof data.replayed !== "boolean"){
validate79.errors = [{instancePath:instancePath+"/replayed",schemaPath:"#/properties/replayed/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
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
if(((data1.card === undefined) && (missing1 = "card")) || ((data1.choices === undefined) && (missing1 = "choices"))){
validate79.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/required",keyword:"required",params:{missingProperty: missing1},message:"must have required property '"+missing1+"'"}];
return false;
}
else {
const _errs6 = errors;
for(const key1 in data1){
if(!((key1 === "card") || (key1 === "choices"))){
validate79.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key1},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs6 === errors){
if(data1.card !== undefined){
const _errs7 = errors;
if(!(validate80(data1.card, {instancePath:instancePath+"/result/card",parentData:data1,parentDataProperty:"card",rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate80.errors : vErrors.concat(validate80.errors);
errors = vErrors.length;
}
var valid1 = _errs7 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.choices !== undefined){
let data3 = data1.choices;
const _errs8 = errors;
if(errors === _errs8){
if(Array.isArray(data3)){
var valid2 = true;
const len0 = data3.length;
for(let i0=0; i0<len0; i0++){
const _errs10 = errors;
if(!(validate84(data3[i0], {instancePath:instancePath+"/result/choices/" + i0,parentData:data3,parentDataProperty:i0,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate84.errors : vErrors.concat(validate84.errors);
errors = vErrors.length;
}
var valid2 = _errs10 === errors;
if(!valid2){
break;
}
}
}
else {
validate79.errors = [{instancePath:instancePath+"/result/choices",schemaPath:"#/properties/result/properties/choices/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs8 === errors;
}
else {
var valid1 = true;
}
}
}
}
}
else {
validate79.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/type",keyword:"type",params:{type: "object"},message:"must be object"}];
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
let data5 = data.status;
const _errs11 = errors;
if(typeof data5 !== "string"){
validate79.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data5 === "ok")){
validate79.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/enum",keyword:"enum",params:{allowedValues: schema44.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs11 === errors;
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
validate79.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate79.errors = vErrors;
return errors === 0;
}
validate79.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate78(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate78.evaluated;
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
if(!(validate79(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate79.errors : vErrors.concat(validate79.errors);
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
validate78.errors = vErrors;
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
validate78.errors = vErrors;
evaluated0.props = props0;
return errors === 0;
}
validate78.evaluated = {"dynamicProps":true,"dynamicItems":false};

export const CommandResponse_CardTags = validate89;
const schema93 = {"oneOf":[{"$ref":"#/$defs/CommandSuccess_CardTags"},{"$ref":"#/$defs/CommandFailure"}],"description":"Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.\nA success confirms the original completion, not the current object state.\nConstruct success only AFTER the storage transaction commits. Cancellation,\na lost response or uncertain commit must never be converted to success."};
const schema94 = {"type":"object","required":["status","result","replayed"],"properties":{"replayed":{"type":"boolean"},"result":{"type":"object","required":["card","assignments"],"properties":{"assignments":{"type":"array","items":{"$ref":"#/$defs/TagAssignment"}},"card":{"$ref":"#/$defs/CardSnapshot"}},"additionalProperties":false},"status":{"$ref":"#/$defs/SuccessStatus"}},"additionalProperties":false};
const schema95 = {"type":"object","required":["tag","values"],"properties":{"tag":{"type":"string"},"values":{"type":"array","items":{"type":"boolean"}}},"additionalProperties":false};

function validate90(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate90.evaluated;
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
validate90.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((key0 === "replayed") || (key0 === "result")) || (key0 === "status"))){
validate90.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.replayed !== undefined){
const _errs2 = errors;
if(typeof data.replayed !== "boolean"){
validate90.errors = [{instancePath:instancePath+"/replayed",schemaPath:"#/properties/replayed/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
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
if(((data1.card === undefined) && (missing1 = "card")) || ((data1.assignments === undefined) && (missing1 = "assignments"))){
validate90.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/required",keyword:"required",params:{missingProperty: missing1},message:"must have required property '"+missing1+"'"}];
return false;
}
else {
const _errs6 = errors;
for(const key1 in data1){
if(!((key1 === "assignments") || (key1 === "card"))){
validate90.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key1},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs6 === errors){
if(data1.assignments !== undefined){
let data2 = data1.assignments;
const _errs7 = errors;
if(errors === _errs7){
if(Array.isArray(data2)){
var valid2 = true;
const len0 = data2.length;
for(let i0=0; i0<len0; i0++){
let data3 = data2[i0];
const _errs9 = errors;
const _errs10 = errors;
if(errors === _errs10){
if(data3 && typeof data3 == "object" && !Array.isArray(data3)){
let missing2;
if(((data3.tag === undefined) && (missing2 = "tag")) || ((data3.values === undefined) && (missing2 = "values"))){
validate90.errors = [{instancePath:instancePath+"/result/assignments/" + i0,schemaPath:"#/$defs/TagAssignment/required",keyword:"required",params:{missingProperty: missing2},message:"must have required property '"+missing2+"'"}];
return false;
}
else {
const _errs12 = errors;
for(const key2 in data3){
if(!((key2 === "tag") || (key2 === "values"))){
validate90.errors = [{instancePath:instancePath+"/result/assignments/" + i0,schemaPath:"#/$defs/TagAssignment/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key2},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs12 === errors){
if(data3.tag !== undefined){
const _errs13 = errors;
if(typeof data3.tag !== "string"){
validate90.errors = [{instancePath:instancePath+"/result/assignments/" + i0+"/tag",schemaPath:"#/$defs/TagAssignment/properties/tag/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid4 = _errs13 === errors;
}
else {
var valid4 = true;
}
if(valid4){
if(data3.values !== undefined){
let data5 = data3.values;
const _errs15 = errors;
if(errors === _errs15){
if(Array.isArray(data5)){
var valid5 = true;
const len1 = data5.length;
for(let i1=0; i1<len1; i1++){
const _errs17 = errors;
if(typeof data5[i1] !== "boolean"){
validate90.errors = [{instancePath:instancePath+"/result/assignments/" + i0+"/values/" + i1,schemaPath:"#/$defs/TagAssignment/properties/values/items/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid5 = _errs17 === errors;
if(!valid5){
break;
}
}
}
else {
validate90.errors = [{instancePath:instancePath+"/result/assignments/" + i0+"/values",schemaPath:"#/$defs/TagAssignment/properties/values/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid4 = _errs15 === errors;
}
else {
var valid4 = true;
}
}
}
}
}
else {
validate90.errors = [{instancePath:instancePath+"/result/assignments/" + i0,schemaPath:"#/$defs/TagAssignment/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
var valid2 = _errs9 === errors;
if(!valid2){
break;
}
}
}
else {
validate90.errors = [{instancePath:instancePath+"/result/assignments",schemaPath:"#/properties/result/properties/assignments/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs7 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.card !== undefined){
const _errs19 = errors;
if(!(validate80(data1.card, {instancePath:instancePath+"/result/card",parentData:data1,parentDataProperty:"card",rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate80.errors : vErrors.concat(validate80.errors);
errors = vErrors.length;
}
var valid1 = _errs19 === errors;
}
else {
var valid1 = true;
}
}
}
}
}
else {
validate90.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/type",keyword:"type",params:{type: "object"},message:"must be object"}];
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
const _errs20 = errors;
if(typeof data8 !== "string"){
validate90.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data8 === "ok")){
validate90.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/enum",keyword:"enum",params:{allowedValues: schema44.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs20 === errors;
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
validate90.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate90.errors = vErrors;
return errors === 0;
}
validate90.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate89(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate89.evaluated;
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
if(!(validate90(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate90.errors : vErrors.concat(validate90.errors);
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
validate89.errors = vErrors;
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
validate89.errors = vErrors;
evaluated0.props = props0;
return errors === 0;
}
validate89.evaluated = {"dynamicProps":true,"dynamicItems":false};

export const CommandResponse_CardList = validate94;
const schema97 = {"oneOf":[{"$ref":"#/$defs/CommandSuccess_CardList"},{"$ref":"#/$defs/CommandFailure"}],"description":"Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.\nA success confirms the original completion, not the current object state.\nConstruct success only AFTER the storage transaction commits. Cancellation,\na lost response or uncertain commit must never be converted to success."};
const schema98 = {"type":"object","required":["status","result","replayed"],"properties":{"replayed":{"type":"boolean"},"result":{"type":"object","required":["cards"],"properties":{"cards":{"type":"array","items":{"$ref":"#/$defs/CardSnapshot"}}},"additionalProperties":false},"status":{"$ref":"#/$defs/SuccessStatus"}},"additionalProperties":false};

function validate95(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate95.evaluated;
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
validate95.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((key0 === "replayed") || (key0 === "result")) || (key0 === "status"))){
validate95.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.replayed !== undefined){
const _errs2 = errors;
if(typeof data.replayed !== "boolean"){
validate95.errors = [{instancePath:instancePath+"/replayed",schemaPath:"#/properties/replayed/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
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
validate95.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/required",keyword:"required",params:{missingProperty: missing1},message:"must have required property '"+missing1+"'"}];
return false;
}
else {
const _errs6 = errors;
for(const key1 in data1){
if(!(key1 === "cards")){
validate95.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key1},message:"must NOT have additional properties"}];
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
if(!(validate80(data2[i0], {instancePath:instancePath+"/result/cards/" + i0,parentData:data2,parentDataProperty:i0,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate80.errors : vErrors.concat(validate80.errors);
errors = vErrors.length;
}
var valid2 = _errs9 === errors;
if(!valid2){
break;
}
}
}
else {
validate95.errors = [{instancePath:instancePath+"/result/cards",schemaPath:"#/properties/result/properties/cards/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
}
}
}
}
else {
validate95.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/type",keyword:"type",params:{type: "object"},message:"must be object"}];
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
validate95.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data4 === "ok")){
validate95.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/enum",keyword:"enum",params:{allowedValues: schema44.enum},message:"must be equal to one of the allowed values"}];
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
validate95.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate95.errors = vErrors;
return errors === 0;
}
validate95.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate94(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate94.evaluated;
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
if(!(validate95(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate95.errors : vErrors.concat(validate95.errors);
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
validate94.errors = vErrors;
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
validate94.errors = vErrors;
evaluated0.props = props0;
return errors === 0;
}
validate94.evaluated = {"dynamicProps":true,"dynamicItems":false};

export const CommandResponse_SearchResults = validate99;
const schema100 = {"oneOf":[{"$ref":"#/$defs/CommandSuccess_SearchResults"},{"$ref":"#/$defs/CommandFailure"}],"description":"Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.\nA success confirms the original completion, not the current object state.\nConstruct success only AFTER the storage transaction commits. Cancellation,\na lost response or uncertain commit must never be converted to success."};
const schema101 = {"type":"object","required":["status","result","replayed"],"properties":{"replayed":{"type":"boolean"},"result":{"type":"object","required":["spaces"],"properties":{"spaces":{"type":"array","items":{"$ref":"#/$defs/SearchSpace"},"description":"Only spaces containing matches, in the same order as space.list."}},"additionalProperties":false},"status":{"$ref":"#/$defs/SuccessStatus"}},"additionalProperties":false};
const schema102 = {"type":"object","required":["space","hits"],"properties":{"hits":{"type":"array","items":{"$ref":"#/$defs/SearchHit"}},"space":{"$ref":"#/$defs/SpaceSnapshot","description":"All current names; len > 1 identifies the leading name-conflict group."}},"additionalProperties":false};
const schema103 = {"type":"object","required":["card","placement_conflict"],"properties":{"card":{"$ref":"#/$defs/CardSnapshot"},"placement_conflict":{"type":"boolean","description":"These hits precede ordinary board positions inside their space."}},"additionalProperties":false};

function validate102(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate102.evaluated;
if(evaluated0.dynamicProps){
evaluated0.props = undefined;
}
if(evaluated0.dynamicItems){
evaluated0.items = undefined;
}
if(errors === 0){
if(data && typeof data == "object" && !Array.isArray(data)){
let missing0;
if(((data.card === undefined) && (missing0 = "card")) || ((data.placement_conflict === undefined) && (missing0 = "placement_conflict"))){
validate102.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!((key0 === "card") || (key0 === "placement_conflict"))){
validate102.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.card !== undefined){
const _errs2 = errors;
if(!(validate80(data.card, {instancePath:instancePath+"/card",parentData:data,parentDataProperty:"card",rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate80.errors : vErrors.concat(validate80.errors);
errors = vErrors.length;
}
var valid0 = _errs2 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.placement_conflict !== undefined){
const _errs3 = errors;
if(typeof data.placement_conflict !== "boolean"){
validate102.errors = [{instancePath:instancePath+"/placement_conflict",schemaPath:"#/properties/placement_conflict/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
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
validate102.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate102.errors = vErrors;
return errors === 0;
}
validate102.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};

const schema104 = {"type":"object","required":["space","names","state","deletion_groups","empty","version"],"properties":{"contents":{"oneOf":[{"$ref":"#/$defs/SpaceContents","description":"Authoritative scope preview. None blocks lifecycle changes."},{"type":"null"}]},"deletion_groups":{"type":"array","items":{"type":"string"}},"empty":{"type":"boolean"},"names":{"type":"array","items":{"type":"string"},"description":"All distinct current names; multiple names require conflict resolution."},"space":{"type":"string"},"state":{"$ref":"#/$defs/SpaceState"},"version":{"$ref":"#/$defs/StateVersion"}},"additionalProperties":false};
const schema105 = {"type":"object","required":["columns","cards","tags"],"properties":{"cards":{"type":"array","items":{"$ref":"#/$defs/CardSnapshot"}},"columns":{"type":"array","items":{"$ref":"#/$defs/ColumnSnapshot"}},"tags":{"type":"array","items":{"$ref":"#/$defs/TagSnapshot"}}},"additionalProperties":false};
const schema106 = {"type":"object","required":["space","column","names","state","deletion_groups","positions","empty","version"],"properties":{"column":{"type":"string"},"contents":{"type":["array","null"],"items":{"$ref":"#/$defs/CardSnapshot"},"description":"Current members, including earlier trash and conflicts, for scope preview.\nNone means unsupported contents; legacy receipts also omit this field."},"deletion_groups":{"type":"array","items":{"type":"string"}},"empty":{"type":"boolean","description":"True only when there are no cards or unknown contents."},"names":{"type":"array","items":{"type":"string"}},"positions":{"type":"array","items":{"$ref":"#/$defs/Rank"},"description":"All distinct position registers; conflicting values are never overwritten."},"space":{"type":"string"},"state":{"$ref":"#/$defs/SpaceState"},"version":{"$ref":"#/$defs/StateVersion"}},"additionalProperties":false};

function validate108(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate108.evaluated;
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
validate108.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(func1.call(schema106.properties, key0))){
validate108.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.column !== undefined){
const _errs2 = errors;
if(typeof data.column !== "string"){
validate108.errors = [{instancePath:instancePath+"/column",schemaPath:"#/properties/column/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid0 = _errs2 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.contents !== undefined){
let data1 = data.contents;
const _errs4 = errors;
if((!(Array.isArray(data1))) && (data1 !== null)){
validate108.errors = [{instancePath:instancePath+"/contents",schemaPath:"#/properties/contents/type",keyword:"type",params:{type: schema106.properties.contents.type},message:"must be array,null"}];
return false;
}
if(errors === _errs4){
if(Array.isArray(data1)){
var valid1 = true;
const len0 = data1.length;
for(let i0=0; i0<len0; i0++){
const _errs6 = errors;
if(!(validate80(data1[i0], {instancePath:instancePath+"/contents/" + i0,parentData:data1,parentDataProperty:i0,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate80.errors : vErrors.concat(validate80.errors);
errors = vErrors.length;
}
var valid1 = _errs6 === errors;
if(!valid1){
break;
}
}
}
}
var valid0 = _errs4 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.deletion_groups !== undefined){
let data3 = data.deletion_groups;
const _errs7 = errors;
if(errors === _errs7){
if(Array.isArray(data3)){
var valid2 = true;
const len1 = data3.length;
for(let i1=0; i1<len1; i1++){
const _errs9 = errors;
if(typeof data3[i1] !== "string"){
validate108.errors = [{instancePath:instancePath+"/deletion_groups/" + i1,schemaPath:"#/properties/deletion_groups/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid2 = _errs9 === errors;
if(!valid2){
break;
}
}
}
else {
validate108.errors = [{instancePath:instancePath+"/deletion_groups",schemaPath:"#/properties/deletion_groups/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid0 = _errs7 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.empty !== undefined){
const _errs11 = errors;
if(typeof data.empty !== "boolean"){
validate108.errors = [{instancePath:instancePath+"/empty",schemaPath:"#/properties/empty/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid0 = _errs11 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.names !== undefined){
let data6 = data.names;
const _errs13 = errors;
if(errors === _errs13){
if(Array.isArray(data6)){
var valid3 = true;
const len2 = data6.length;
for(let i2=0; i2<len2; i2++){
const _errs15 = errors;
if(typeof data6[i2] !== "string"){
validate108.errors = [{instancePath:instancePath+"/names/" + i2,schemaPath:"#/properties/names/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid3 = _errs15 === errors;
if(!valid3){
break;
}
}
}
else {
validate108.errors = [{instancePath:instancePath+"/names",schemaPath:"#/properties/names/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid0 = _errs13 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.positions !== undefined){
let data8 = data.positions;
const _errs17 = errors;
if(errors === _errs17){
if(Array.isArray(data8)){
var valid4 = true;
const len3 = data8.length;
for(let i3=0; i3<len3; i3++){
const _errs19 = errors;
if(!(validate68(data8[i3], {instancePath:instancePath+"/positions/" + i3,parentData:data8,parentDataProperty:i3,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate68.errors : vErrors.concat(validate68.errors);
errors = vErrors.length;
}
var valid4 = _errs19 === errors;
if(!valid4){
break;
}
}
}
else {
validate108.errors = [{instancePath:instancePath+"/positions",schemaPath:"#/properties/positions/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid0 = _errs17 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.space !== undefined){
const _errs20 = errors;
if(typeof data.space !== "string"){
validate108.errors = [{instancePath:instancePath+"/space",schemaPath:"#/properties/space/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid0 = _errs20 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.state !== undefined){
let data11 = data.state;
const _errs22 = errors;
if(typeof data11 !== "string"){
validate108.errors = [{instancePath:instancePath+"/state",schemaPath:"#/$defs/SpaceState/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(((data11 === "active") || (data11 === "trashed")) || (data11 === "conflicted"))){
validate108.errors = [{instancePath:instancePath+"/state",schemaPath:"#/$defs/SpaceState/enum",keyword:"enum",params:{allowedValues: schema78.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs22 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.version !== undefined){
let data12 = data.version;
const _errs25 = errors;
const _errs26 = errors;
if(errors === _errs26){
if(Array.isArray(data12)){
if(data12.length > 32){
validate108.errors = [{instancePath:instancePath+"/version",schemaPath:"#/$defs/StateVersion/maxItems",keyword:"maxItems",params:{limit: 32},message:"must NOT have more than 32 items"}];
return false;
}
else {
if(data12.length < 32){
validate108.errors = [{instancePath:instancePath+"/version",schemaPath:"#/$defs/StateVersion/minItems",keyword:"minItems",params:{limit: 32},message:"must NOT have fewer than 32 items"}];
return false;
}
else {
var valid7 = true;
const len4 = data12.length;
for(let i4=0; i4<len4; i4++){
let data13 = data12[i4];
const _errs28 = errors;
if(!((typeof data13 == "number") && (!(data13 % 1) && !isNaN(data13)))){
validate108.errors = [{instancePath:instancePath+"/version/" + i4,schemaPath:"#/$defs/StateVersion/items/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs28){
if(typeof data13 == "number"){
if(data13 > 255 || isNaN(data13)){
validate108.errors = [{instancePath:instancePath+"/version/" + i4,schemaPath:"#/$defs/StateVersion/items/maximum",keyword:"maximum",params:{comparison: "<=", limit: 255},message:"must be <= 255"}];
return false;
}
else {
if(data13 < 0 || isNaN(data13)){
validate108.errors = [{instancePath:instancePath+"/version/" + i4,schemaPath:"#/$defs/StateVersion/items/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
}
var valid7 = _errs28 === errors;
if(!valid7){
break;
}
}
}
}
}
else {
validate108.errors = [{instancePath:instancePath+"/version",schemaPath:"#/$defs/StateVersion/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid0 = _errs25 === errors;
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
else {
validate108.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate108.errors = vErrors;
return errors === 0;
}
validate108.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};

const schema109 = {"type":"object","required":["space","tag","names","state","version"],"properties":{"deletion_conflict":{"type":"boolean","description":"True when deletion competes with independent assignment or a different deletion decision. Resolve with tag.resolve_deletion and a fresh version; ordinary writes remain blocked."},"name_group":{"type":"array","items":{"type":"string"}},"names":{"type":"array","items":{"type":"string"}},"space":{"type":"string"},"state":{"$ref":"#/$defs/TagState"},"tag":{"type":"string"},"version":{"$ref":"#/$defs/StateVersion","description":"Includes causal assignment history, so deletion cannot expand silently."}},"additionalProperties":false};
const schema110 = {"type":"string","enum":["active","deleted","conflicted"]};

function validate112(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate112.evaluated;
if(evaluated0.dynamicProps){
evaluated0.props = undefined;
}
if(evaluated0.dynamicItems){
evaluated0.items = undefined;
}
if(errors === 0){
if(data && typeof data == "object" && !Array.isArray(data)){
let missing0;
if((((((data.space === undefined) && (missing0 = "space")) || ((data.tag === undefined) && (missing0 = "tag"))) || ((data.names === undefined) && (missing0 = "names"))) || ((data.state === undefined) && (missing0 = "state"))) || ((data.version === undefined) && (missing0 = "version"))){
validate112.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((((((key0 === "deletion_conflict") || (key0 === "name_group")) || (key0 === "names")) || (key0 === "space")) || (key0 === "state")) || (key0 === "tag")) || (key0 === "version"))){
validate112.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.deletion_conflict !== undefined){
const _errs2 = errors;
if(typeof data.deletion_conflict !== "boolean"){
validate112.errors = [{instancePath:instancePath+"/deletion_conflict",schemaPath:"#/properties/deletion_conflict/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid0 = _errs2 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.name_group !== undefined){
let data1 = data.name_group;
const _errs4 = errors;
if(errors === _errs4){
if(Array.isArray(data1)){
var valid1 = true;
const len0 = data1.length;
for(let i0=0; i0<len0; i0++){
const _errs6 = errors;
if(typeof data1[i0] !== "string"){
validate112.errors = [{instancePath:instancePath+"/name_group/" + i0,schemaPath:"#/properties/name_group/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid1 = _errs6 === errors;
if(!valid1){
break;
}
}
}
else {
validate112.errors = [{instancePath:instancePath+"/name_group",schemaPath:"#/properties/name_group/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid0 = _errs4 === errors;
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
validate112.errors = [{instancePath:instancePath+"/names/" + i1,schemaPath:"#/properties/names/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid2 = _errs10 === errors;
if(!valid2){
break;
}
}
}
else {
validate112.errors = [{instancePath:instancePath+"/names",schemaPath:"#/properties/names/type",keyword:"type",params:{type: "array"},message:"must be array"}];
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
validate112.errors = [{instancePath:instancePath+"/space",schemaPath:"#/properties/space/type",keyword:"type",params:{type: "string"},message:"must be string"}];
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
validate112.errors = [{instancePath:instancePath+"/state",schemaPath:"#/$defs/TagState/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(((data6 === "active") || (data6 === "deleted")) || (data6 === "conflicted"))){
validate112.errors = [{instancePath:instancePath+"/state",schemaPath:"#/$defs/TagState/enum",keyword:"enum",params:{allowedValues: schema110.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs14 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.tag !== undefined){
const _errs17 = errors;
if(typeof data.tag !== "string"){
validate112.errors = [{instancePath:instancePath+"/tag",schemaPath:"#/properties/tag/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid0 = _errs17 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.version !== undefined){
let data8 = data.version;
const _errs19 = errors;
const _errs20 = errors;
if(errors === _errs20){
if(Array.isArray(data8)){
if(data8.length > 32){
validate112.errors = [{instancePath:instancePath+"/version",schemaPath:"#/$defs/StateVersion/maxItems",keyword:"maxItems",params:{limit: 32},message:"must NOT have more than 32 items"}];
return false;
}
else {
if(data8.length < 32){
validate112.errors = [{instancePath:instancePath+"/version",schemaPath:"#/$defs/StateVersion/minItems",keyword:"minItems",params:{limit: 32},message:"must NOT have fewer than 32 items"}];
return false;
}
else {
var valid5 = true;
const len2 = data8.length;
for(let i2=0; i2<len2; i2++){
let data9 = data8[i2];
const _errs22 = errors;
if(!((typeof data9 == "number") && (!(data9 % 1) && !isNaN(data9)))){
validate112.errors = [{instancePath:instancePath+"/version/" + i2,schemaPath:"#/$defs/StateVersion/items/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs22){
if(typeof data9 == "number"){
if(data9 > 255 || isNaN(data9)){
validate112.errors = [{instancePath:instancePath+"/version/" + i2,schemaPath:"#/$defs/StateVersion/items/maximum",keyword:"maximum",params:{comparison: "<=", limit: 255},message:"must be <= 255"}];
return false;
}
else {
if(data9 < 0 || isNaN(data9)){
validate112.errors = [{instancePath:instancePath+"/version/" + i2,schemaPath:"#/$defs/StateVersion/items/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
}
var valid5 = _errs22 === errors;
if(!valid5){
break;
}
}
}
}
}
else {
validate112.errors = [{instancePath:instancePath+"/version",schemaPath:"#/$defs/StateVersion/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
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
}
}
}
}
else {
validate112.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate112.errors = vErrors;
return errors === 0;
}
validate112.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate106(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate106.evaluated;
if(evaluated0.dynamicProps){
evaluated0.props = undefined;
}
if(evaluated0.dynamicItems){
evaluated0.items = undefined;
}
if(errors === 0){
if(data && typeof data == "object" && !Array.isArray(data)){
let missing0;
if((((data.columns === undefined) && (missing0 = "columns")) || ((data.cards === undefined) && (missing0 = "cards"))) || ((data.tags === undefined) && (missing0 = "tags"))){
validate106.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((key0 === "cards") || (key0 === "columns")) || (key0 === "tags"))){
validate106.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.cards !== undefined){
let data0 = data.cards;
const _errs2 = errors;
if(errors === _errs2){
if(Array.isArray(data0)){
var valid1 = true;
const len0 = data0.length;
for(let i0=0; i0<len0; i0++){
const _errs4 = errors;
if(!(validate80(data0[i0], {instancePath:instancePath+"/cards/" + i0,parentData:data0,parentDataProperty:i0,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate80.errors : vErrors.concat(validate80.errors);
errors = vErrors.length;
}
var valid1 = _errs4 === errors;
if(!valid1){
break;
}
}
}
else {
validate106.errors = [{instancePath:instancePath+"/cards",schemaPath:"#/properties/cards/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid0 = _errs2 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.columns !== undefined){
let data2 = data.columns;
const _errs5 = errors;
if(errors === _errs5){
if(Array.isArray(data2)){
var valid2 = true;
const len1 = data2.length;
for(let i1=0; i1<len1; i1++){
const _errs7 = errors;
if(!(validate108(data2[i1], {instancePath:instancePath+"/columns/" + i1,parentData:data2,parentDataProperty:i1,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate108.errors : vErrors.concat(validate108.errors);
errors = vErrors.length;
}
var valid2 = _errs7 === errors;
if(!valid2){
break;
}
}
}
else {
validate106.errors = [{instancePath:instancePath+"/columns",schemaPath:"#/properties/columns/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid0 = _errs5 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.tags !== undefined){
let data4 = data.tags;
const _errs8 = errors;
if(errors === _errs8){
if(Array.isArray(data4)){
var valid3 = true;
const len2 = data4.length;
for(let i2=0; i2<len2; i2++){
const _errs10 = errors;
if(!(validate112(data4[i2], {instancePath:instancePath+"/tags/" + i2,parentData:data4,parentDataProperty:i2,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate112.errors : vErrors.concat(validate112.errors);
errors = vErrors.length;
}
var valid3 = _errs10 === errors;
if(!valid3){
break;
}
}
}
else {
validate106.errors = [{instancePath:instancePath+"/tags",schemaPath:"#/properties/tags/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid0 = _errs8 === errors;
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
validate106.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate106.errors = vErrors;
return errors === 0;
}
validate106.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate105(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate105.evaluated;
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
validate105.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((((((key0 === "contents") || (key0 === "deletion_groups")) || (key0 === "empty")) || (key0 === "names")) || (key0 === "space")) || (key0 === "state")) || (key0 === "version"))){
validate105.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.contents !== undefined){
let data0 = data.contents;
const _errs2 = errors;
const _errs3 = errors;
let valid1 = false;
let passing0 = null;
const _errs4 = errors;
if(!(validate106(data0, {instancePath:instancePath+"/contents",parentData:data,parentDataProperty:"contents",rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate106.errors : vErrors.concat(validate106.errors);
errors = vErrors.length;
}
var _valid0 = _errs4 === errors;
if(_valid0){
valid1 = true;
passing0 = 0;
}
const _errs5 = errors;
if(data0 !== null){
const err0 = {instancePath:instancePath+"/contents",schemaPath:"#/properties/contents/oneOf/1/type",keyword:"type",params:{type: "null"},message:"must be null"};
if(vErrors === null){
vErrors = [err0];
}
else {
vErrors.push(err0);
}
errors++;
}
var _valid0 = _errs5 === errors;
if(_valid0 && valid1){
valid1 = false;
passing0 = [passing0, 1];
}
else {
if(_valid0){
valid1 = true;
passing0 = 1;
}
}
if(!valid1){
const err1 = {instancePath:instancePath+"/contents",schemaPath:"#/properties/contents/oneOf",keyword:"oneOf",params:{passingSchemas: passing0},message:"must match exactly one schema in oneOf"};
if(vErrors === null){
vErrors = [err1];
}
else {
vErrors.push(err1);
}
errors++;
validate105.errors = vErrors;
return false;
}
else {
errors = _errs3;
if(vErrors !== null){
if(_errs3){
vErrors.length = _errs3;
}
else {
vErrors = null;
}
}
}
var valid0 = _errs2 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.deletion_groups !== undefined){
let data1 = data.deletion_groups;
const _errs7 = errors;
if(errors === _errs7){
if(Array.isArray(data1)){
var valid2 = true;
const len0 = data1.length;
for(let i0=0; i0<len0; i0++){
const _errs9 = errors;
if(typeof data1[i0] !== "string"){
validate105.errors = [{instancePath:instancePath+"/deletion_groups/" + i0,schemaPath:"#/properties/deletion_groups/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid2 = _errs9 === errors;
if(!valid2){
break;
}
}
}
else {
validate105.errors = [{instancePath:instancePath+"/deletion_groups",schemaPath:"#/properties/deletion_groups/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid0 = _errs7 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.empty !== undefined){
const _errs11 = errors;
if(typeof data.empty !== "boolean"){
validate105.errors = [{instancePath:instancePath+"/empty",schemaPath:"#/properties/empty/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid0 = _errs11 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.names !== undefined){
let data4 = data.names;
const _errs13 = errors;
if(errors === _errs13){
if(Array.isArray(data4)){
var valid3 = true;
const len1 = data4.length;
for(let i1=0; i1<len1; i1++){
const _errs15 = errors;
if(typeof data4[i1] !== "string"){
validate105.errors = [{instancePath:instancePath+"/names/" + i1,schemaPath:"#/properties/names/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid3 = _errs15 === errors;
if(!valid3){
break;
}
}
}
else {
validate105.errors = [{instancePath:instancePath+"/names",schemaPath:"#/properties/names/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid0 = _errs13 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.space !== undefined){
const _errs17 = errors;
if(typeof data.space !== "string"){
validate105.errors = [{instancePath:instancePath+"/space",schemaPath:"#/properties/space/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid0 = _errs17 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.state !== undefined){
let data7 = data.state;
const _errs19 = errors;
if(typeof data7 !== "string"){
validate105.errors = [{instancePath:instancePath+"/state",schemaPath:"#/$defs/SpaceState/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(((data7 === "active") || (data7 === "trashed")) || (data7 === "conflicted"))){
validate105.errors = [{instancePath:instancePath+"/state",schemaPath:"#/$defs/SpaceState/enum",keyword:"enum",params:{allowedValues: schema78.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs19 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.version !== undefined){
let data8 = data.version;
const _errs22 = errors;
const _errs23 = errors;
if(errors === _errs23){
if(Array.isArray(data8)){
if(data8.length > 32){
validate105.errors = [{instancePath:instancePath+"/version",schemaPath:"#/$defs/StateVersion/maxItems",keyword:"maxItems",params:{limit: 32},message:"must NOT have more than 32 items"}];
return false;
}
else {
if(data8.length < 32){
validate105.errors = [{instancePath:instancePath+"/version",schemaPath:"#/$defs/StateVersion/minItems",keyword:"minItems",params:{limit: 32},message:"must NOT have fewer than 32 items"}];
return false;
}
else {
var valid6 = true;
const len2 = data8.length;
for(let i2=0; i2<len2; i2++){
let data9 = data8[i2];
const _errs25 = errors;
if(!((typeof data9 == "number") && (!(data9 % 1) && !isNaN(data9)))){
validate105.errors = [{instancePath:instancePath+"/version/" + i2,schemaPath:"#/$defs/StateVersion/items/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs25){
if(typeof data9 == "number"){
if(data9 > 255 || isNaN(data9)){
validate105.errors = [{instancePath:instancePath+"/version/" + i2,schemaPath:"#/$defs/StateVersion/items/maximum",keyword:"maximum",params:{comparison: "<=", limit: 255},message:"must be <= 255"}];
return false;
}
else {
if(data9 < 0 || isNaN(data9)){
validate105.errors = [{instancePath:instancePath+"/version/" + i2,schemaPath:"#/$defs/StateVersion/items/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
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
validate105.errors = [{instancePath:instancePath+"/version",schemaPath:"#/$defs/StateVersion/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid0 = _errs22 === errors;
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
else {
validate105.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate105.errors = vErrors;
return errors === 0;
}
validate105.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate101(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate101.evaluated;
if(evaluated0.dynamicProps){
evaluated0.props = undefined;
}
if(evaluated0.dynamicItems){
evaluated0.items = undefined;
}
if(errors === 0){
if(data && typeof data == "object" && !Array.isArray(data)){
let missing0;
if(((data.space === undefined) && (missing0 = "space")) || ((data.hits === undefined) && (missing0 = "hits"))){
validate101.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!((key0 === "hits") || (key0 === "space"))){
validate101.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.hits !== undefined){
let data0 = data.hits;
const _errs2 = errors;
if(errors === _errs2){
if(Array.isArray(data0)){
var valid1 = true;
const len0 = data0.length;
for(let i0=0; i0<len0; i0++){
const _errs4 = errors;
if(!(validate102(data0[i0], {instancePath:instancePath+"/hits/" + i0,parentData:data0,parentDataProperty:i0,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate102.errors : vErrors.concat(validate102.errors);
errors = vErrors.length;
}
var valid1 = _errs4 === errors;
if(!valid1){
break;
}
}
}
else {
validate101.errors = [{instancePath:instancePath+"/hits",schemaPath:"#/properties/hits/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid0 = _errs2 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.space !== undefined){
const _errs5 = errors;
if(!(validate105(data.space, {instancePath:instancePath+"/space",parentData:data,parentDataProperty:"space",rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate105.errors : vErrors.concat(validate105.errors);
errors = vErrors.length;
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
validate101.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate101.errors = vErrors;
return errors === 0;
}
validate101.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate100(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate100.evaluated;
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
validate100.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((key0 === "replayed") || (key0 === "result")) || (key0 === "status"))){
validate100.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.replayed !== undefined){
const _errs2 = errors;
if(typeof data.replayed !== "boolean"){
validate100.errors = [{instancePath:instancePath+"/replayed",schemaPath:"#/properties/replayed/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
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
validate100.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/required",keyword:"required",params:{missingProperty: missing1},message:"must have required property '"+missing1+"'"}];
return false;
}
else {
const _errs6 = errors;
for(const key1 in data1){
if(!(key1 === "spaces")){
validate100.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key1},message:"must NOT have additional properties"}];
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
if(!(validate101(data2[i0], {instancePath:instancePath+"/result/spaces/" + i0,parentData:data2,parentDataProperty:i0,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate101.errors : vErrors.concat(validate101.errors);
errors = vErrors.length;
}
var valid2 = _errs9 === errors;
if(!valid2){
break;
}
}
}
else {
validate100.errors = [{instancePath:instancePath+"/result/spaces",schemaPath:"#/properties/result/properties/spaces/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
}
}
}
}
else {
validate100.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/type",keyword:"type",params:{type: "object"},message:"must be object"}];
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
validate100.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data4 === "ok")){
validate100.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/enum",keyword:"enum",params:{allowedValues: schema44.enum},message:"must be equal to one of the allowed values"}];
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
validate100.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate100.errors = vErrors;
return errors === 0;
}
validate100.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate99(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate99.evaluated;
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
if(!(validate100(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate100.errors : vErrors.concat(validate100.errors);
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
validate99.errors = vErrors;
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
validate99.errors = vErrors;
evaluated0.props = props0;
return errors === 0;
}
validate99.evaluated = {"dynamicProps":true,"dynamicItems":false};

export const CommandResponse_ColumnSnapshot = validate119;
const schema115 = {"oneOf":[{"$ref":"#/$defs/CommandSuccess_ColumnSnapshot"},{"$ref":"#/$defs/CommandFailure"}],"description":"Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.\nA success confirms the original completion, not the current object state.\nConstruct success only AFTER the storage transaction commits. Cancellation,\na lost response or uncertain commit must never be converted to success."};
const schema116 = {"type":"object","required":["status","result","replayed"],"properties":{"replayed":{"type":"boolean"},"result":{"type":"object","required":["space","column","names","state","deletion_groups","positions","empty","version"],"properties":{"column":{"type":"string"},"contents":{"type":["array","null"],"items":{"$ref":"#/$defs/CardSnapshot"},"description":"Current members, including earlier trash and conflicts, for scope preview.\nNone means unsupported contents; legacy receipts also omit this field."},"deletion_groups":{"type":"array","items":{"type":"string"}},"empty":{"type":"boolean","description":"True only when there are no cards or unknown contents."},"names":{"type":"array","items":{"type":"string"}},"positions":{"type":"array","items":{"$ref":"#/$defs/Rank"},"description":"All distinct position registers; conflicting values are never overwritten."},"space":{"type":"string"},"state":{"$ref":"#/$defs/SpaceState"},"version":{"$ref":"#/$defs/StateVersion"}},"additionalProperties":false},"status":{"$ref":"#/$defs/SuccessStatus"}},"additionalProperties":false};

function validate120(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate120.evaluated;
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
validate120.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((key0 === "replayed") || (key0 === "result")) || (key0 === "status"))){
validate120.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.replayed !== undefined){
const _errs2 = errors;
if(typeof data.replayed !== "boolean"){
validate120.errors = [{instancePath:instancePath+"/replayed",schemaPath:"#/properties/replayed/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
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
validate120.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/required",keyword:"required",params:{missingProperty: missing1},message:"must have required property '"+missing1+"'"}];
return false;
}
else {
const _errs6 = errors;
for(const key1 in data1){
if(!(func1.call(schema116.properties.result.properties, key1))){
validate120.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key1},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs6 === errors){
if(data1.column !== undefined){
const _errs7 = errors;
if(typeof data1.column !== "string"){
validate120.errors = [{instancePath:instancePath+"/result/column",schemaPath:"#/properties/result/properties/column/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid1 = _errs7 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.contents !== undefined){
let data3 = data1.contents;
const _errs9 = errors;
if((!(Array.isArray(data3))) && (data3 !== null)){
validate120.errors = [{instancePath:instancePath+"/result/contents",schemaPath:"#/properties/result/properties/contents/type",keyword:"type",params:{type: schema116.properties.result.properties.contents.type},message:"must be array,null"}];
return false;
}
if(errors === _errs9){
if(Array.isArray(data3)){
var valid2 = true;
const len0 = data3.length;
for(let i0=0; i0<len0; i0++){
const _errs11 = errors;
if(!(validate80(data3[i0], {instancePath:instancePath+"/result/contents/" + i0,parentData:data3,parentDataProperty:i0,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate80.errors : vErrors.concat(validate80.errors);
errors = vErrors.length;
}
var valid2 = _errs11 === errors;
if(!valid2){
break;
}
}
}
}
var valid1 = _errs9 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.deletion_groups !== undefined){
let data5 = data1.deletion_groups;
const _errs12 = errors;
if(errors === _errs12){
if(Array.isArray(data5)){
var valid3 = true;
const len1 = data5.length;
for(let i1=0; i1<len1; i1++){
const _errs14 = errors;
if(typeof data5[i1] !== "string"){
validate120.errors = [{instancePath:instancePath+"/result/deletion_groups/" + i1,schemaPath:"#/properties/result/properties/deletion_groups/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid3 = _errs14 === errors;
if(!valid3){
break;
}
}
}
else {
validate120.errors = [{instancePath:instancePath+"/result/deletion_groups",schemaPath:"#/properties/result/properties/deletion_groups/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs12 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.empty !== undefined){
const _errs16 = errors;
if(typeof data1.empty !== "boolean"){
validate120.errors = [{instancePath:instancePath+"/result/empty",schemaPath:"#/properties/result/properties/empty/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid1 = _errs16 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.names !== undefined){
let data8 = data1.names;
const _errs18 = errors;
if(errors === _errs18){
if(Array.isArray(data8)){
var valid4 = true;
const len2 = data8.length;
for(let i2=0; i2<len2; i2++){
const _errs20 = errors;
if(typeof data8[i2] !== "string"){
validate120.errors = [{instancePath:instancePath+"/result/names/" + i2,schemaPath:"#/properties/result/properties/names/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid4 = _errs20 === errors;
if(!valid4){
break;
}
}
}
else {
validate120.errors = [{instancePath:instancePath+"/result/names",schemaPath:"#/properties/result/properties/names/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs18 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.positions !== undefined){
let data10 = data1.positions;
const _errs22 = errors;
if(errors === _errs22){
if(Array.isArray(data10)){
var valid5 = true;
const len3 = data10.length;
for(let i3=0; i3<len3; i3++){
const _errs24 = errors;
if(!(validate68(data10[i3], {instancePath:instancePath+"/result/positions/" + i3,parentData:data10,parentDataProperty:i3,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate68.errors : vErrors.concat(validate68.errors);
errors = vErrors.length;
}
var valid5 = _errs24 === errors;
if(!valid5){
break;
}
}
}
else {
validate120.errors = [{instancePath:instancePath+"/result/positions",schemaPath:"#/properties/result/properties/positions/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs22 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.space !== undefined){
const _errs25 = errors;
if(typeof data1.space !== "string"){
validate120.errors = [{instancePath:instancePath+"/result/space",schemaPath:"#/properties/result/properties/space/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid1 = _errs25 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.state !== undefined){
let data13 = data1.state;
const _errs27 = errors;
if(typeof data13 !== "string"){
validate120.errors = [{instancePath:instancePath+"/result/state",schemaPath:"#/$defs/SpaceState/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(((data13 === "active") || (data13 === "trashed")) || (data13 === "conflicted"))){
validate120.errors = [{instancePath:instancePath+"/result/state",schemaPath:"#/$defs/SpaceState/enum",keyword:"enum",params:{allowedValues: schema78.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid1 = _errs27 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.version !== undefined){
let data14 = data1.version;
const _errs30 = errors;
const _errs31 = errors;
if(errors === _errs31){
if(Array.isArray(data14)){
if(data14.length > 32){
validate120.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/maxItems",keyword:"maxItems",params:{limit: 32},message:"must NOT have more than 32 items"}];
return false;
}
else {
if(data14.length < 32){
validate120.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/minItems",keyword:"minItems",params:{limit: 32},message:"must NOT have fewer than 32 items"}];
return false;
}
else {
var valid8 = true;
const len4 = data14.length;
for(let i4=0; i4<len4; i4++){
let data15 = data14[i4];
const _errs33 = errors;
if(!((typeof data15 == "number") && (!(data15 % 1) && !isNaN(data15)))){
validate120.errors = [{instancePath:instancePath+"/result/version/" + i4,schemaPath:"#/$defs/StateVersion/items/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs33){
if(typeof data15 == "number"){
if(data15 > 255 || isNaN(data15)){
validate120.errors = [{instancePath:instancePath+"/result/version/" + i4,schemaPath:"#/$defs/StateVersion/items/maximum",keyword:"maximum",params:{comparison: "<=", limit: 255},message:"must be <= 255"}];
return false;
}
else {
if(data15 < 0 || isNaN(data15)){
validate120.errors = [{instancePath:instancePath+"/result/version/" + i4,schemaPath:"#/$defs/StateVersion/items/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
}
var valid8 = _errs33 === errors;
if(!valid8){
break;
}
}
}
}
}
else {
validate120.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs30 === errors;
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
else {
validate120.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/type",keyword:"type",params:{type: "object"},message:"must be object"}];
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
let data16 = data.status;
const _errs35 = errors;
if(typeof data16 !== "string"){
validate120.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data16 === "ok")){
validate120.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/enum",keyword:"enum",params:{allowedValues: schema44.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs35 === errors;
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
validate120.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate120.errors = vErrors;
return errors === 0;
}
validate120.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate119(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate119.evaluated;
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
if(!(validate120(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate120.errors : vErrors.concat(validate120.errors);
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
validate119.errors = vErrors;
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
validate119.errors = vErrors;
evaluated0.props = props0;
return errors === 0;
}
validate119.evaluated = {"dynamicProps":true,"dynamicItems":false};

export const CommandResponse_ColumnPositions = validate125;
const schema120 = {"oneOf":[{"$ref":"#/$defs/CommandSuccess_ColumnPositions"},{"$ref":"#/$defs/CommandFailure"}],"description":"Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.\nA success confirms the original completion, not the current object state.\nConstruct success only AFTER the storage transaction commits. Cancellation,\na lost response or uncertain commit must never be converted to success."};
const schema121 = {"type":"object","required":["status","result","replayed"],"properties":{"replayed":{"type":"boolean"},"result":{"type":"object","required":["column","choices"],"properties":{"choices":{"type":"array","items":{"$ref":"#/$defs/ColumnPositionChoice"}},"column":{"$ref":"#/$defs/ColumnSnapshot"}},"additionalProperties":false},"status":{"$ref":"#/$defs/SuccessStatus"}},"additionalProperties":false};
const schema122 = {"type":"object","required":["position"],"properties":{"before":{"type":["string","null"]},"position":{"$ref":"#/$defs/Rank"}},"additionalProperties":false};

function validate127(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate127.evaluated;
if(evaluated0.dynamicProps){
evaluated0.props = undefined;
}
if(evaluated0.dynamicItems){
evaluated0.items = undefined;
}
if(errors === 0){
if(data && typeof data == "object" && !Array.isArray(data)){
let missing0;
if((data.position === undefined) && (missing0 = "position")){
validate127.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!((key0 === "before") || (key0 === "position"))){
validate127.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.before !== undefined){
let data0 = data.before;
const _errs2 = errors;
if((typeof data0 !== "string") && (data0 !== null)){
validate127.errors = [{instancePath:instancePath+"/before",schemaPath:"#/properties/before/type",keyword:"type",params:{type: schema122.properties.before.type},message:"must be string,null"}];
return false;
}
var valid0 = _errs2 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.position !== undefined){
const _errs4 = errors;
if(!(validate68(data.position, {instancePath:instancePath+"/position",parentData:data,parentDataProperty:"position",rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate68.errors : vErrors.concat(validate68.errors);
errors = vErrors.length;
}
var valid0 = _errs4 === errors;
}
else {
var valid0 = true;
}
}
}
}
}
else {
validate127.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate127.errors = vErrors;
return errors === 0;
}
validate127.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate126(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate126.evaluated;
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
validate126.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((key0 === "replayed") || (key0 === "result")) || (key0 === "status"))){
validate126.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.replayed !== undefined){
const _errs2 = errors;
if(typeof data.replayed !== "boolean"){
validate126.errors = [{instancePath:instancePath+"/replayed",schemaPath:"#/properties/replayed/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
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
if(((data1.column === undefined) && (missing1 = "column")) || ((data1.choices === undefined) && (missing1 = "choices"))){
validate126.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/required",keyword:"required",params:{missingProperty: missing1},message:"must have required property '"+missing1+"'"}];
return false;
}
else {
const _errs6 = errors;
for(const key1 in data1){
if(!((key1 === "choices") || (key1 === "column"))){
validate126.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key1},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs6 === errors){
if(data1.choices !== undefined){
let data2 = data1.choices;
const _errs7 = errors;
if(errors === _errs7){
if(Array.isArray(data2)){
var valid2 = true;
const len0 = data2.length;
for(let i0=0; i0<len0; i0++){
const _errs9 = errors;
if(!(validate127(data2[i0], {instancePath:instancePath+"/result/choices/" + i0,parentData:data2,parentDataProperty:i0,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate127.errors : vErrors.concat(validate127.errors);
errors = vErrors.length;
}
var valid2 = _errs9 === errors;
if(!valid2){
break;
}
}
}
else {
validate126.errors = [{instancePath:instancePath+"/result/choices",schemaPath:"#/properties/result/properties/choices/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs7 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.column !== undefined){
const _errs10 = errors;
if(!(validate108(data1.column, {instancePath:instancePath+"/result/column",parentData:data1,parentDataProperty:"column",rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate108.errors : vErrors.concat(validate108.errors);
errors = vErrors.length;
}
var valid1 = _errs10 === errors;
}
else {
var valid1 = true;
}
}
}
}
}
else {
validate126.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/type",keyword:"type",params:{type: "object"},message:"must be object"}];
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
let data5 = data.status;
const _errs11 = errors;
if(typeof data5 !== "string"){
validate126.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data5 === "ok")){
validate126.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/enum",keyword:"enum",params:{allowedValues: schema44.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs11 === errors;
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
validate126.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate126.errors = vErrors;
return errors === 0;
}
validate126.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate125(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate125.evaluated;
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
if(!(validate126(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate126.errors : vErrors.concat(validate126.errors);
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
validate125.errors = vErrors;
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
validate125.errors = vErrors;
evaluated0.props = props0;
return errors === 0;
}
validate125.evaluated = {"dynamicProps":true,"dynamicItems":false};

export const CommandResponse_ColumnList = validate133;
const schema124 = {"oneOf":[{"$ref":"#/$defs/CommandSuccess_ColumnList"},{"$ref":"#/$defs/CommandFailure"}],"description":"Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.\nA success confirms the original completion, not the current object state.\nConstruct success only AFTER the storage transaction commits. Cancellation,\na lost response or uncertain commit must never be converted to success."};
const schema125 = {"type":"object","required":["status","result","replayed"],"properties":{"replayed":{"type":"boolean"},"result":{"type":"object","required":["columns"],"properties":{"columns":{"type":"array","items":{"$ref":"#/$defs/ColumnSnapshot"}}},"additionalProperties":false},"status":{"$ref":"#/$defs/SuccessStatus"}},"additionalProperties":false};

function validate134(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate134.evaluated;
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
validate134.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((key0 === "replayed") || (key0 === "result")) || (key0 === "status"))){
validate134.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.replayed !== undefined){
const _errs2 = errors;
if(typeof data.replayed !== "boolean"){
validate134.errors = [{instancePath:instancePath+"/replayed",schemaPath:"#/properties/replayed/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
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
validate134.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/required",keyword:"required",params:{missingProperty: missing1},message:"must have required property '"+missing1+"'"}];
return false;
}
else {
const _errs6 = errors;
for(const key1 in data1){
if(!(key1 === "columns")){
validate134.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key1},message:"must NOT have additional properties"}];
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
if(!(validate108(data2[i0], {instancePath:instancePath+"/result/columns/" + i0,parentData:data2,parentDataProperty:i0,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate108.errors : vErrors.concat(validate108.errors);
errors = vErrors.length;
}
var valid2 = _errs9 === errors;
if(!valid2){
break;
}
}
}
else {
validate134.errors = [{instancePath:instancePath+"/result/columns",schemaPath:"#/properties/result/properties/columns/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
}
}
}
}
else {
validate134.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/type",keyword:"type",params:{type: "object"},message:"must be object"}];
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
validate134.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data4 === "ok")){
validate134.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/enum",keyword:"enum",params:{allowedValues: schema44.enum},message:"must be equal to one of the allowed values"}];
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
validate134.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate134.errors = vErrors;
return errors === 0;
}
validate134.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate133(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate133.evaluated;
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
if(!(validate134(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate134.errors : vErrors.concat(validate134.errors);
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
validate133.errors = vErrors;
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
validate133.errors = vErrors;
evaluated0.props = props0;
return errors === 0;
}
validate133.evaluated = {"dynamicProps":true,"dynamicItems":false};

export const CommandResponse_HistoryPreview = validate138;
const schema127 = {"oneOf":[{"$ref":"#/$defs/CommandSuccess_HistoryPreview"},{"$ref":"#/$defs/CommandFailure"}],"description":"Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.\nA success confirms the original completion, not the current object state.\nConstruct success only AFTER the storage transaction commits. Cancellation,\na lost response or uncertain commit must never be converted to success."};
const schema128 = {"type":"object","required":["status","result","replayed"],"properties":{"replayed":{"type":"boolean"},"result":{"type":"object","required":["space","entries","text_bytes","removable_bytes","version"],"properties":{"card":{"type":["string","null"],"description":"None means every card in this space, including trash."},"entries":{"type":"array","items":{"$ref":"#/$defs/HistoryEntry"}},"removable_bytes":{"type":"integer","format":"int64","minimum":0},"space":{"type":"string"},"text_bytes":{"type":"integer","format":"int64","description":"Logical UTF-8 title/description bytes, not allocated database space.","minimum":0},"version":{"$ref":"#/$defs/StateVersion"}},"additionalProperties":false},"status":{"$ref":"#/$defs/SuccessStatus"}},"additionalProperties":false};
const schema129 = {"type":"object","required":["card","version","retained"],"properties":{"card":{"type":"string"},"retained":{"type":"boolean","description":"Conservatively retain history of every card with unresolved conflicts."},"version":{"$ref":"#/$defs/CardTextVersion"}},"additionalProperties":false};

function validate140(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate140.evaluated;
if(evaluated0.dynamicProps){
evaluated0.props = undefined;
}
if(evaluated0.dynamicItems){
evaluated0.items = undefined;
}
if(errors === 0){
if(data && typeof data == "object" && !Array.isArray(data)){
let missing0;
if((((data.card === undefined) && (missing0 = "card")) || ((data.version === undefined) && (missing0 = "version"))) || ((data.retained === undefined) && (missing0 = "retained"))){
validate140.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((key0 === "card") || (key0 === "retained")) || (key0 === "version"))){
validate140.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.card !== undefined){
const _errs2 = errors;
if(typeof data.card !== "string"){
validate140.errors = [{instancePath:instancePath+"/card",schemaPath:"#/properties/card/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid0 = _errs2 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.retained !== undefined){
const _errs4 = errors;
if(typeof data.retained !== "boolean"){
validate140.errors = [{instancePath:instancePath+"/retained",schemaPath:"#/properties/retained/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid0 = _errs4 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.version !== undefined){
let data2 = data.version;
const _errs6 = errors;
const _errs7 = errors;
if(errors === _errs7){
if(data2 && typeof data2 == "object" && !Array.isArray(data2)){
let missing1;
if((((((data2.id === undefined) && (missing1 = "id")) || ((data2.subject === undefined) && (missing1 = "subject"))) || ((data2.sequence === undefined) && (missing1 = "sequence"))) || ((data2.title === undefined) && (missing1 = "title"))) || ((data2.description === undefined) && (missing1 = "description"))){
validate140.errors = [{instancePath:instancePath+"/version",schemaPath:"#/$defs/CardTextVersion/required",keyword:"required",params:{missingProperty: missing1},message:"must have required property '"+missing1+"'"}];
return false;
}
else {
const _errs9 = errors;
for(const key1 in data2){
if(!(((((key1 === "description") || (key1 === "id")) || (key1 === "sequence")) || (key1 === "subject")) || (key1 === "title"))){
validate140.errors = [{instancePath:instancePath+"/version",schemaPath:"#/$defs/CardTextVersion/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key1},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs9 === errors){
if(data2.description !== undefined){
const _errs10 = errors;
if(typeof data2.description !== "string"){
validate140.errors = [{instancePath:instancePath+"/version/description",schemaPath:"#/$defs/CardTextVersion/properties/description/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid2 = _errs10 === errors;
}
else {
var valid2 = true;
}
if(valid2){
if(data2.id !== undefined){
const _errs12 = errors;
if(typeof data2.id !== "string"){
validate140.errors = [{instancePath:instancePath+"/version/id",schemaPath:"#/$defs/CardTextVersion/properties/id/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid2 = _errs12 === errors;
}
else {
var valid2 = true;
}
if(valid2){
if(data2.sequence !== undefined){
let data5 = data2.sequence;
const _errs14 = errors;
if(!((typeof data5 == "number") && (!(data5 % 1) && !isNaN(data5)))){
validate140.errors = [{instancePath:instancePath+"/version/sequence",schemaPath:"#/$defs/CardTextVersion/properties/sequence/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs14){
if(typeof data5 == "number"){
if(data5 < 0 || isNaN(data5)){
validate140.errors = [{instancePath:instancePath+"/version/sequence",schemaPath:"#/$defs/CardTextVersion/properties/sequence/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
var valid2 = _errs14 === errors;
}
else {
var valid2 = true;
}
if(valid2){
if(data2.subject !== undefined){
const _errs16 = errors;
if(typeof data2.subject !== "string"){
validate140.errors = [{instancePath:instancePath+"/version/subject",schemaPath:"#/$defs/CardTextVersion/properties/subject/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid2 = _errs16 === errors;
}
else {
var valid2 = true;
}
if(valid2){
if(data2.title !== undefined){
const _errs18 = errors;
if(typeof data2.title !== "string"){
validate140.errors = [{instancePath:instancePath+"/version/title",schemaPath:"#/$defs/CardTextVersion/properties/title/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid2 = _errs18 === errors;
}
else {
var valid2 = true;
}
}
}
}
}
}
}
}
else {
validate140.errors = [{instancePath:instancePath+"/version",schemaPath:"#/$defs/CardTextVersion/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
var valid0 = _errs6 === errors;
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
validate140.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate140.errors = vErrors;
return errors === 0;
}
validate140.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate139(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate139.evaluated;
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
validate139.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((key0 === "replayed") || (key0 === "result")) || (key0 === "status"))){
validate139.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.replayed !== undefined){
const _errs2 = errors;
if(typeof data.replayed !== "boolean"){
validate139.errors = [{instancePath:instancePath+"/replayed",schemaPath:"#/properties/replayed/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
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
if((((((data1.space === undefined) && (missing1 = "space")) || ((data1.entries === undefined) && (missing1 = "entries"))) || ((data1.text_bytes === undefined) && (missing1 = "text_bytes"))) || ((data1.removable_bytes === undefined) && (missing1 = "removable_bytes"))) || ((data1.version === undefined) && (missing1 = "version"))){
validate139.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/required",keyword:"required",params:{missingProperty: missing1},message:"must have required property '"+missing1+"'"}];
return false;
}
else {
const _errs6 = errors;
for(const key1 in data1){
if(!((((((key1 === "card") || (key1 === "entries")) || (key1 === "removable_bytes")) || (key1 === "space")) || (key1 === "text_bytes")) || (key1 === "version"))){
validate139.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key1},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs6 === errors){
if(data1.card !== undefined){
let data2 = data1.card;
const _errs7 = errors;
if((typeof data2 !== "string") && (data2 !== null)){
validate139.errors = [{instancePath:instancePath+"/result/card",schemaPath:"#/properties/result/properties/card/type",keyword:"type",params:{type: schema128.properties.result.properties.card.type},message:"must be string,null"}];
return false;
}
var valid1 = _errs7 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.entries !== undefined){
let data3 = data1.entries;
const _errs9 = errors;
if(errors === _errs9){
if(Array.isArray(data3)){
var valid2 = true;
const len0 = data3.length;
for(let i0=0; i0<len0; i0++){
const _errs11 = errors;
if(!(validate140(data3[i0], {instancePath:instancePath+"/result/entries/" + i0,parentData:data3,parentDataProperty:i0,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate140.errors : vErrors.concat(validate140.errors);
errors = vErrors.length;
}
var valid2 = _errs11 === errors;
if(!valid2){
break;
}
}
}
else {
validate139.errors = [{instancePath:instancePath+"/result/entries",schemaPath:"#/properties/result/properties/entries/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs9 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.removable_bytes !== undefined){
let data5 = data1.removable_bytes;
const _errs12 = errors;
if(!((typeof data5 == "number") && (!(data5 % 1) && !isNaN(data5)))){
validate139.errors = [{instancePath:instancePath+"/result/removable_bytes",schemaPath:"#/properties/result/properties/removable_bytes/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs12){
if(typeof data5 == "number"){
if(data5 < 0 || isNaN(data5)){
validate139.errors = [{instancePath:instancePath+"/result/removable_bytes",schemaPath:"#/properties/result/properties/removable_bytes/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
var valid1 = _errs12 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.space !== undefined){
const _errs14 = errors;
if(typeof data1.space !== "string"){
validate139.errors = [{instancePath:instancePath+"/result/space",schemaPath:"#/properties/result/properties/space/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid1 = _errs14 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.text_bytes !== undefined){
let data7 = data1.text_bytes;
const _errs16 = errors;
if(!((typeof data7 == "number") && (!(data7 % 1) && !isNaN(data7)))){
validate139.errors = [{instancePath:instancePath+"/result/text_bytes",schemaPath:"#/properties/result/properties/text_bytes/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs16){
if(typeof data7 == "number"){
if(data7 < 0 || isNaN(data7)){
validate139.errors = [{instancePath:instancePath+"/result/text_bytes",schemaPath:"#/properties/result/properties/text_bytes/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
var valid1 = _errs16 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.version !== undefined){
let data8 = data1.version;
const _errs18 = errors;
const _errs19 = errors;
if(errors === _errs19){
if(Array.isArray(data8)){
if(data8.length > 32){
validate139.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/maxItems",keyword:"maxItems",params:{limit: 32},message:"must NOT have more than 32 items"}];
return false;
}
else {
if(data8.length < 32){
validate139.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/minItems",keyword:"minItems",params:{limit: 32},message:"must NOT have fewer than 32 items"}];
return false;
}
else {
var valid4 = true;
const len1 = data8.length;
for(let i1=0; i1<len1; i1++){
let data9 = data8[i1];
const _errs21 = errors;
if(!((typeof data9 == "number") && (!(data9 % 1) && !isNaN(data9)))){
validate139.errors = [{instancePath:instancePath+"/result/version/" + i1,schemaPath:"#/$defs/StateVersion/items/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs21){
if(typeof data9 == "number"){
if(data9 > 255 || isNaN(data9)){
validate139.errors = [{instancePath:instancePath+"/result/version/" + i1,schemaPath:"#/$defs/StateVersion/items/maximum",keyword:"maximum",params:{comparison: "<=", limit: 255},message:"must be <= 255"}];
return false;
}
else {
if(data9 < 0 || isNaN(data9)){
validate139.errors = [{instancePath:instancePath+"/result/version/" + i1,schemaPath:"#/$defs/StateVersion/items/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
}
var valid4 = _errs21 === errors;
if(!valid4){
break;
}
}
}
}
}
else {
validate139.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/type",keyword:"type",params:{type: "array"},message:"must be array"}];
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
}
else {
validate139.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/type",keyword:"type",params:{type: "object"},message:"must be object"}];
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
let data10 = data.status;
const _errs23 = errors;
if(typeof data10 !== "string"){
validate139.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data10 === "ok")){
validate139.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/enum",keyword:"enum",params:{allowedValues: schema44.enum},message:"must be equal to one of the allowed values"}];
return false;
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
else {
validate139.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate139.errors = vErrors;
return errors === 0;
}
validate139.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate138(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate138.evaluated;
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
if(!(validate139(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate139.errors : vErrors.concat(validate139.errors);
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
validate138.errors = vErrors;
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
validate138.errors = vErrors;
evaluated0.props = props0;
return errors === 0;
}
validate138.evaluated = {"dynamicProps":true,"dynamicItems":false};

export const CommandResponse_InvitationSnapshot = validate144;
const schema133 = {"oneOf":[{"$ref":"#/$defs/CommandSuccess_InvitationSnapshot"},{"$ref":"#/$defs/CommandFailure"}],"description":"Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.\nA success confirms the original completion, not the current object state.\nConstruct success only AFTER the storage transaction commits. Cancellation,\na lost response or uncertain commit must never be converted to success."};
const schema134 = {"type":"object","required":["status","result","replayed"],"properties":{"replayed":{"type":"boolean"},"result":{"type":"object","required":["invitation","created_at_unix_ms","expires_at_unix_ms","status","version"],"properties":{"created_at_unix_ms":{"type":"integer","format":"int64","maximum":253402300799999,"minimum":0},"expires_at_unix_ms":{"type":"integer","format":"int64","maximum":253402300799999,"minimum":0},"invitation":{"type":"string"},"status":{"$ref":"#/$defs/InvitationStatus"},"version":{"$ref":"#/$defs/StateVersion"}},"additionalProperties":false},"status":{"$ref":"#/$defs/SuccessStatus"}},"additionalProperties":false};
const schema135 = {"type":"string","enum":["open","expired","cancelled","used","locked"]};

function validate145(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate145.evaluated;
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
validate145.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((key0 === "replayed") || (key0 === "result")) || (key0 === "status"))){
validate145.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.replayed !== undefined){
const _errs2 = errors;
if(typeof data.replayed !== "boolean"){
validate145.errors = [{instancePath:instancePath+"/replayed",schemaPath:"#/properties/replayed/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
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
if((((((data1.invitation === undefined) && (missing1 = "invitation")) || ((data1.created_at_unix_ms === undefined) && (missing1 = "created_at_unix_ms"))) || ((data1.expires_at_unix_ms === undefined) && (missing1 = "expires_at_unix_ms"))) || ((data1.status === undefined) && (missing1 = "status"))) || ((data1.version === undefined) && (missing1 = "version"))){
validate145.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/required",keyword:"required",params:{missingProperty: missing1},message:"must have required property '"+missing1+"'"}];
return false;
}
else {
const _errs6 = errors;
for(const key1 in data1){
if(!(((((key1 === "created_at_unix_ms") || (key1 === "expires_at_unix_ms")) || (key1 === "invitation")) || (key1 === "status")) || (key1 === "version"))){
validate145.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key1},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs6 === errors){
if(data1.created_at_unix_ms !== undefined){
let data2 = data1.created_at_unix_ms;
const _errs7 = errors;
if(!((typeof data2 == "number") && (!(data2 % 1) && !isNaN(data2)))){
validate145.errors = [{instancePath:instancePath+"/result/created_at_unix_ms",schemaPath:"#/properties/result/properties/created_at_unix_ms/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs7){
if(typeof data2 == "number"){
if(data2 > 253402300799999 || isNaN(data2)){
validate145.errors = [{instancePath:instancePath+"/result/created_at_unix_ms",schemaPath:"#/properties/result/properties/created_at_unix_ms/maximum",keyword:"maximum",params:{comparison: "<=", limit: 253402300799999},message:"must be <= 253402300799999"}];
return false;
}
else {
if(data2 < 0 || isNaN(data2)){
validate145.errors = [{instancePath:instancePath+"/result/created_at_unix_ms",schemaPath:"#/properties/result/properties/created_at_unix_ms/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
}
var valid1 = _errs7 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.expires_at_unix_ms !== undefined){
let data3 = data1.expires_at_unix_ms;
const _errs9 = errors;
if(!((typeof data3 == "number") && (!(data3 % 1) && !isNaN(data3)))){
validate145.errors = [{instancePath:instancePath+"/result/expires_at_unix_ms",schemaPath:"#/properties/result/properties/expires_at_unix_ms/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs9){
if(typeof data3 == "number"){
if(data3 > 253402300799999 || isNaN(data3)){
validate145.errors = [{instancePath:instancePath+"/result/expires_at_unix_ms",schemaPath:"#/properties/result/properties/expires_at_unix_ms/maximum",keyword:"maximum",params:{comparison: "<=", limit: 253402300799999},message:"must be <= 253402300799999"}];
return false;
}
else {
if(data3 < 0 || isNaN(data3)){
validate145.errors = [{instancePath:instancePath+"/result/expires_at_unix_ms",schemaPath:"#/properties/result/properties/expires_at_unix_ms/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
}
var valid1 = _errs9 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.invitation !== undefined){
const _errs11 = errors;
if(typeof data1.invitation !== "string"){
validate145.errors = [{instancePath:instancePath+"/result/invitation",schemaPath:"#/properties/result/properties/invitation/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid1 = _errs11 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.status !== undefined){
let data5 = data1.status;
const _errs13 = errors;
if(typeof data5 !== "string"){
validate145.errors = [{instancePath:instancePath+"/result/status",schemaPath:"#/$defs/InvitationStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(((((data5 === "open") || (data5 === "expired")) || (data5 === "cancelled")) || (data5 === "used")) || (data5 === "locked"))){
validate145.errors = [{instancePath:instancePath+"/result/status",schemaPath:"#/$defs/InvitationStatus/enum",keyword:"enum",params:{allowedValues: schema135.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid1 = _errs13 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.version !== undefined){
let data6 = data1.version;
const _errs16 = errors;
const _errs17 = errors;
if(errors === _errs17){
if(Array.isArray(data6)){
if(data6.length > 32){
validate145.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/maxItems",keyword:"maxItems",params:{limit: 32},message:"must NOT have more than 32 items"}];
return false;
}
else {
if(data6.length < 32){
validate145.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/minItems",keyword:"minItems",params:{limit: 32},message:"must NOT have fewer than 32 items"}];
return false;
}
else {
var valid4 = true;
const len0 = data6.length;
for(let i0=0; i0<len0; i0++){
let data7 = data6[i0];
const _errs19 = errors;
if(!((typeof data7 == "number") && (!(data7 % 1) && !isNaN(data7)))){
validate145.errors = [{instancePath:instancePath+"/result/version/" + i0,schemaPath:"#/$defs/StateVersion/items/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs19){
if(typeof data7 == "number"){
if(data7 > 255 || isNaN(data7)){
validate145.errors = [{instancePath:instancePath+"/result/version/" + i0,schemaPath:"#/$defs/StateVersion/items/maximum",keyword:"maximum",params:{comparison: "<=", limit: 255},message:"must be <= 255"}];
return false;
}
else {
if(data7 < 0 || isNaN(data7)){
validate145.errors = [{instancePath:instancePath+"/result/version/" + i0,schemaPath:"#/$defs/StateVersion/items/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
}
var valid4 = _errs19 === errors;
if(!valid4){
break;
}
}
}
}
}
else {
validate145.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs16 === errors;
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
validate145.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/type",keyword:"type",params:{type: "object"},message:"must be object"}];
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
const _errs21 = errors;
if(typeof data8 !== "string"){
validate145.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data8 === "ok")){
validate145.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/enum",keyword:"enum",params:{allowedValues: schema44.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs21 === errors;
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
validate145.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate145.errors = vErrors;
return errors === 0;
}
validate145.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate144(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate144.evaluated;
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
if(!(validate145(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate145.errors : vErrors.concat(validate145.errors);
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
validate144.errors = vErrors;
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
validate144.errors = vErrors;
evaluated0.props = props0;
return errors === 0;
}
validate144.evaluated = {"dynamicProps":true,"dynamicItems":false};

export const CommandResponse_InvitationList = validate148;
const schema138 = {"oneOf":[{"$ref":"#/$defs/CommandSuccess_InvitationList"},{"$ref":"#/$defs/CommandFailure"}],"description":"Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.\nA success confirms the original completion, not the current object state.\nConstruct success only AFTER the storage transaction commits. Cancellation,\na lost response or uncertain commit must never be converted to success."};
const schema139 = {"type":"object","required":["status","result","replayed"],"properties":{"replayed":{"type":"boolean"},"result":{"type":"object","required":["entries"],"properties":{"entries":{"type":"array","items":{"$ref":"#/$defs/InvitationSnapshot"},"maxItems":50},"next":{"type":["string","null"]}},"additionalProperties":false},"status":{"$ref":"#/$defs/SuccessStatus"}},"additionalProperties":false};
const schema140 = {"type":"object","required":["invitation","created_at_unix_ms","expires_at_unix_ms","status","version"],"properties":{"created_at_unix_ms":{"type":"integer","format":"int64","maximum":253402300799999,"minimum":0},"expires_at_unix_ms":{"type":"integer","format":"int64","maximum":253402300799999,"minimum":0},"invitation":{"type":"string"},"status":{"$ref":"#/$defs/InvitationStatus"},"version":{"$ref":"#/$defs/StateVersion"}},"additionalProperties":false};

function validate150(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate150.evaluated;
if(evaluated0.dynamicProps){
evaluated0.props = undefined;
}
if(evaluated0.dynamicItems){
evaluated0.items = undefined;
}
if(errors === 0){
if(data && typeof data == "object" && !Array.isArray(data)){
let missing0;
if((((((data.invitation === undefined) && (missing0 = "invitation")) || ((data.created_at_unix_ms === undefined) && (missing0 = "created_at_unix_ms"))) || ((data.expires_at_unix_ms === undefined) && (missing0 = "expires_at_unix_ms"))) || ((data.status === undefined) && (missing0 = "status"))) || ((data.version === undefined) && (missing0 = "version"))){
validate150.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((((key0 === "created_at_unix_ms") || (key0 === "expires_at_unix_ms")) || (key0 === "invitation")) || (key0 === "status")) || (key0 === "version"))){
validate150.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.created_at_unix_ms !== undefined){
let data0 = data.created_at_unix_ms;
const _errs2 = errors;
if(!((typeof data0 == "number") && (!(data0 % 1) && !isNaN(data0)))){
validate150.errors = [{instancePath:instancePath+"/created_at_unix_ms",schemaPath:"#/properties/created_at_unix_ms/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs2){
if(typeof data0 == "number"){
if(data0 > 253402300799999 || isNaN(data0)){
validate150.errors = [{instancePath:instancePath+"/created_at_unix_ms",schemaPath:"#/properties/created_at_unix_ms/maximum",keyword:"maximum",params:{comparison: "<=", limit: 253402300799999},message:"must be <= 253402300799999"}];
return false;
}
else {
if(data0 < 0 || isNaN(data0)){
validate150.errors = [{instancePath:instancePath+"/created_at_unix_ms",schemaPath:"#/properties/created_at_unix_ms/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
}
var valid0 = _errs2 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.expires_at_unix_ms !== undefined){
let data1 = data.expires_at_unix_ms;
const _errs4 = errors;
if(!((typeof data1 == "number") && (!(data1 % 1) && !isNaN(data1)))){
validate150.errors = [{instancePath:instancePath+"/expires_at_unix_ms",schemaPath:"#/properties/expires_at_unix_ms/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs4){
if(typeof data1 == "number"){
if(data1 > 253402300799999 || isNaN(data1)){
validate150.errors = [{instancePath:instancePath+"/expires_at_unix_ms",schemaPath:"#/properties/expires_at_unix_ms/maximum",keyword:"maximum",params:{comparison: "<=", limit: 253402300799999},message:"must be <= 253402300799999"}];
return false;
}
else {
if(data1 < 0 || isNaN(data1)){
validate150.errors = [{instancePath:instancePath+"/expires_at_unix_ms",schemaPath:"#/properties/expires_at_unix_ms/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
}
var valid0 = _errs4 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.invitation !== undefined){
const _errs6 = errors;
if(typeof data.invitation !== "string"){
validate150.errors = [{instancePath:instancePath+"/invitation",schemaPath:"#/properties/invitation/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid0 = _errs6 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.status !== undefined){
let data3 = data.status;
const _errs8 = errors;
if(typeof data3 !== "string"){
validate150.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/InvitationStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(((((data3 === "open") || (data3 === "expired")) || (data3 === "cancelled")) || (data3 === "used")) || (data3 === "locked"))){
validate150.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/InvitationStatus/enum",keyword:"enum",params:{allowedValues: schema135.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs8 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.version !== undefined){
let data4 = data.version;
const _errs11 = errors;
const _errs12 = errors;
if(errors === _errs12){
if(Array.isArray(data4)){
if(data4.length > 32){
validate150.errors = [{instancePath:instancePath+"/version",schemaPath:"#/$defs/StateVersion/maxItems",keyword:"maxItems",params:{limit: 32},message:"must NOT have more than 32 items"}];
return false;
}
else {
if(data4.length < 32){
validate150.errors = [{instancePath:instancePath+"/version",schemaPath:"#/$defs/StateVersion/minItems",keyword:"minItems",params:{limit: 32},message:"must NOT have fewer than 32 items"}];
return false;
}
else {
var valid3 = true;
const len0 = data4.length;
for(let i0=0; i0<len0; i0++){
let data5 = data4[i0];
const _errs14 = errors;
if(!((typeof data5 == "number") && (!(data5 % 1) && !isNaN(data5)))){
validate150.errors = [{instancePath:instancePath+"/version/" + i0,schemaPath:"#/$defs/StateVersion/items/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs14){
if(typeof data5 == "number"){
if(data5 > 255 || isNaN(data5)){
validate150.errors = [{instancePath:instancePath+"/version/" + i0,schemaPath:"#/$defs/StateVersion/items/maximum",keyword:"maximum",params:{comparison: "<=", limit: 255},message:"must be <= 255"}];
return false;
}
else {
if(data5 < 0 || isNaN(data5)){
validate150.errors = [{instancePath:instancePath+"/version/" + i0,schemaPath:"#/$defs/StateVersion/items/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
}
var valid3 = _errs14 === errors;
if(!valid3){
break;
}
}
}
}
}
else {
validate150.errors = [{instancePath:instancePath+"/version",schemaPath:"#/$defs/StateVersion/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid0 = _errs11 === errors;
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
else {
validate150.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate150.errors = vErrors;
return errors === 0;
}
validate150.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate149(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate149.evaluated;
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
validate149.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((key0 === "replayed") || (key0 === "result")) || (key0 === "status"))){
validate149.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.replayed !== undefined){
const _errs2 = errors;
if(typeof data.replayed !== "boolean"){
validate149.errors = [{instancePath:instancePath+"/replayed",schemaPath:"#/properties/replayed/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
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
if((data1.entries === undefined) && (missing1 = "entries")){
validate149.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/required",keyword:"required",params:{missingProperty: missing1},message:"must have required property '"+missing1+"'"}];
return false;
}
else {
const _errs6 = errors;
for(const key1 in data1){
if(!((key1 === "entries") || (key1 === "next"))){
validate149.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key1},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs6 === errors){
if(data1.entries !== undefined){
let data2 = data1.entries;
const _errs7 = errors;
if(errors === _errs7){
if(Array.isArray(data2)){
if(data2.length > 50){
validate149.errors = [{instancePath:instancePath+"/result/entries",schemaPath:"#/properties/result/properties/entries/maxItems",keyword:"maxItems",params:{limit: 50},message:"must NOT have more than 50 items"}];
return false;
}
else {
var valid2 = true;
const len0 = data2.length;
for(let i0=0; i0<len0; i0++){
const _errs9 = errors;
if(!(validate150(data2[i0], {instancePath:instancePath+"/result/entries/" + i0,parentData:data2,parentDataProperty:i0,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate150.errors : vErrors.concat(validate150.errors);
errors = vErrors.length;
}
var valid2 = _errs9 === errors;
if(!valid2){
break;
}
}
}
}
else {
validate149.errors = [{instancePath:instancePath+"/result/entries",schemaPath:"#/properties/result/properties/entries/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs7 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.next !== undefined){
let data4 = data1.next;
const _errs10 = errors;
if((typeof data4 !== "string") && (data4 !== null)){
validate149.errors = [{instancePath:instancePath+"/result/next",schemaPath:"#/properties/result/properties/next/type",keyword:"type",params:{type: schema139.properties.result.properties.next.type},message:"must be string,null"}];
return false;
}
var valid1 = _errs10 === errors;
}
else {
var valid1 = true;
}
}
}
}
}
else {
validate149.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/type",keyword:"type",params:{type: "object"},message:"must be object"}];
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
let data5 = data.status;
const _errs12 = errors;
if(typeof data5 !== "string"){
validate149.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data5 === "ok")){
validate149.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/enum",keyword:"enum",params:{allowedValues: schema44.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs12 === errors;
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
validate149.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate149.errors = vErrors;
return errors === 0;
}
validate149.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate148(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate148.evaluated;
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
if(!(validate149(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate149.errors : vErrors.concat(validate149.errors);
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
validate148.errors = vErrors;
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
validate148.errors = vErrors;
evaluated0.props = props0;
return errors === 0;
}
validate148.evaluated = {"dynamicProps":true,"dynamicItems":false};

export const CommandResponse_MaintenanceView = validate154;
const schema144 = {"oneOf":[{"$ref":"#/$defs/CommandSuccess_MaintenanceView"},{"$ref":"#/$defs/CommandFailure"}],"description":"Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.\nA success confirms the original completion, not the current object state.\nConstruct success only AFTER the storage transaction commits. Cancellation,\na lost response or uncertain commit must never be converted to success."};
const schema145 = {"type":"object","required":["status","result","replayed"],"properties":{"replayed":{"type":"boolean"},"result":{"type":"object","required":["version","normal_ready","owner","writable","folder_available","copies","problems","access"],"properties":{"access":{"type":"array","items":{"$ref":"#/$defs/MaintenanceAccess"}},"copies":{"type":"array","items":{"$ref":"#/$defs/FullCopy"}},"endpoint":{"type":["string","null"]},"folder":{"type":["string","null"]},"folder_available":{"type":"boolean"},"issued_subject":{"type":["string","null"]},"normal_ready":{"type":"boolean"},"owner":{"type":"boolean"},"plan":{"oneOf":[{"$ref":"#/$defs/RecoveryPlan"},{"type":"null"}]},"problems":{"type":"array","items":{"type":"string"}},"reason":{"type":["string","null"]},"version":{"$ref":"#/$defs/StateVersion"},"writable":{"type":"boolean"}},"additionalProperties":false},"status":{"$ref":"#/$defs/SuccessStatus"}},"additionalProperties":false};
const schema146 = {"type":"object","required":["subject","label","writable","revoked"],"properties":{"expires_ms":{"type":["integer","null"],"format":"int64","minimum":0},"label":{"type":"string"},"revoked":{"type":"boolean"},"subject":{"type":"string"},"writable":{"type":"boolean"}},"additionalProperties":false};
const schema147 = {"type":"object","required":["id","created_at_unix_ms","spaces"],"properties":{"created_at_unix_ms":{"type":"integer","format":"int64","minimum":0},"id":{"type":"string"},"spaces":{"type":"integer","minimum":0}},"additionalProperties":false};
const schema148 = {"type":"object","required":["id","copy","spaces","created_at_unix_ms","phase","problem"],"properties":{"copy":{"type":"string"},"created_at_unix_ms":{"type":"integer","format":"int64","minimum":0},"id":{"type":"string"},"phase":{"$ref":"#/$defs/RecoveryPhase"},"problem":{"type":"string"},"spaces":{"type":"integer","minimum":0}},"additionalProperties":false};
const schema149 = {"type":"string","enum":["prepared","applying","restored","reverting","reverted","cancelled"]};

function validate156(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate156.evaluated;
if(evaluated0.dynamicProps){
evaluated0.props = undefined;
}
if(evaluated0.dynamicItems){
evaluated0.items = undefined;
}
if(errors === 0){
if(data && typeof data == "object" && !Array.isArray(data)){
let missing0;
if(((((((data.id === undefined) && (missing0 = "id")) || ((data.copy === undefined) && (missing0 = "copy"))) || ((data.spaces === undefined) && (missing0 = "spaces"))) || ((data.created_at_unix_ms === undefined) && (missing0 = "created_at_unix_ms"))) || ((data.phase === undefined) && (missing0 = "phase"))) || ((data.problem === undefined) && (missing0 = "problem"))){
validate156.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!((((((key0 === "copy") || (key0 === "created_at_unix_ms")) || (key0 === "id")) || (key0 === "phase")) || (key0 === "problem")) || (key0 === "spaces"))){
validate156.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.copy !== undefined){
const _errs2 = errors;
if(typeof data.copy !== "string"){
validate156.errors = [{instancePath:instancePath+"/copy",schemaPath:"#/properties/copy/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid0 = _errs2 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.created_at_unix_ms !== undefined){
let data1 = data.created_at_unix_ms;
const _errs4 = errors;
if(!((typeof data1 == "number") && (!(data1 % 1) && !isNaN(data1)))){
validate156.errors = [{instancePath:instancePath+"/created_at_unix_ms",schemaPath:"#/properties/created_at_unix_ms/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs4){
if(typeof data1 == "number"){
if(data1 < 0 || isNaN(data1)){
validate156.errors = [{instancePath:instancePath+"/created_at_unix_ms",schemaPath:"#/properties/created_at_unix_ms/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
var valid0 = _errs4 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.id !== undefined){
const _errs6 = errors;
if(typeof data.id !== "string"){
validate156.errors = [{instancePath:instancePath+"/id",schemaPath:"#/properties/id/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid0 = _errs6 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.phase !== undefined){
let data3 = data.phase;
const _errs8 = errors;
if(typeof data3 !== "string"){
validate156.errors = [{instancePath:instancePath+"/phase",schemaPath:"#/$defs/RecoveryPhase/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!((((((data3 === "prepared") || (data3 === "applying")) || (data3 === "restored")) || (data3 === "reverting")) || (data3 === "reverted")) || (data3 === "cancelled"))){
validate156.errors = [{instancePath:instancePath+"/phase",schemaPath:"#/$defs/RecoveryPhase/enum",keyword:"enum",params:{allowedValues: schema149.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs8 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.problem !== undefined){
const _errs11 = errors;
if(typeof data.problem !== "string"){
validate156.errors = [{instancePath:instancePath+"/problem",schemaPath:"#/properties/problem/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid0 = _errs11 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.spaces !== undefined){
let data5 = data.spaces;
const _errs13 = errors;
if(!((typeof data5 == "number") && (!(data5 % 1) && !isNaN(data5)))){
validate156.errors = [{instancePath:instancePath+"/spaces",schemaPath:"#/properties/spaces/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs13){
if(typeof data5 == "number"){
if(data5 < 0 || isNaN(data5)){
validate156.errors = [{instancePath:instancePath+"/spaces",schemaPath:"#/properties/spaces/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
var valid0 = _errs13 === errors;
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
validate156.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate156.errors = vErrors;
return errors === 0;
}
validate156.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate155(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate155.evaluated;
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
validate155.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((key0 === "replayed") || (key0 === "result")) || (key0 === "status"))){
validate155.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.replayed !== undefined){
const _errs2 = errors;
if(typeof data.replayed !== "boolean"){
validate155.errors = [{instancePath:instancePath+"/replayed",schemaPath:"#/properties/replayed/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
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
if(((((((((data1.version === undefined) && (missing1 = "version")) || ((data1.normal_ready === undefined) && (missing1 = "normal_ready"))) || ((data1.owner === undefined) && (missing1 = "owner"))) || ((data1.writable === undefined) && (missing1 = "writable"))) || ((data1.folder_available === undefined) && (missing1 = "folder_available"))) || ((data1.copies === undefined) && (missing1 = "copies"))) || ((data1.problems === undefined) && (missing1 = "problems"))) || ((data1.access === undefined) && (missing1 = "access"))){
validate155.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/required",keyword:"required",params:{missingProperty: missing1},message:"must have required property '"+missing1+"'"}];
return false;
}
else {
const _errs6 = errors;
for(const key1 in data1){
if(!(func1.call(schema145.properties.result.properties, key1))){
validate155.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key1},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs6 === errors){
if(data1.access !== undefined){
let data2 = data1.access;
const _errs7 = errors;
if(errors === _errs7){
if(Array.isArray(data2)){
var valid2 = true;
const len0 = data2.length;
for(let i0=0; i0<len0; i0++){
let data3 = data2[i0];
const _errs9 = errors;
const _errs10 = errors;
if(errors === _errs10){
if(data3 && typeof data3 == "object" && !Array.isArray(data3)){
let missing2;
if(((((data3.subject === undefined) && (missing2 = "subject")) || ((data3.label === undefined) && (missing2 = "label"))) || ((data3.writable === undefined) && (missing2 = "writable"))) || ((data3.revoked === undefined) && (missing2 = "revoked"))){
validate155.errors = [{instancePath:instancePath+"/result/access/" + i0,schemaPath:"#/$defs/MaintenanceAccess/required",keyword:"required",params:{missingProperty: missing2},message:"must have required property '"+missing2+"'"}];
return false;
}
else {
const _errs12 = errors;
for(const key2 in data3){
if(!(((((key2 === "expires_ms") || (key2 === "label")) || (key2 === "revoked")) || (key2 === "subject")) || (key2 === "writable"))){
validate155.errors = [{instancePath:instancePath+"/result/access/" + i0,schemaPath:"#/$defs/MaintenanceAccess/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key2},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs12 === errors){
if(data3.expires_ms !== undefined){
let data4 = data3.expires_ms;
const _errs13 = errors;
if((!((typeof data4 == "number") && (!(data4 % 1) && !isNaN(data4)))) && (data4 !== null)){
validate155.errors = [{instancePath:instancePath+"/result/access/" + i0+"/expires_ms",schemaPath:"#/$defs/MaintenanceAccess/properties/expires_ms/type",keyword:"type",params:{type: schema146.properties.expires_ms.type},message:"must be integer,null"}];
return false;
}
if(errors === _errs13){
if(typeof data4 == "number"){
if(data4 < 0 || isNaN(data4)){
validate155.errors = [{instancePath:instancePath+"/result/access/" + i0+"/expires_ms",schemaPath:"#/$defs/MaintenanceAccess/properties/expires_ms/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
var valid4 = _errs13 === errors;
}
else {
var valid4 = true;
}
if(valid4){
if(data3.label !== undefined){
const _errs15 = errors;
if(typeof data3.label !== "string"){
validate155.errors = [{instancePath:instancePath+"/result/access/" + i0+"/label",schemaPath:"#/$defs/MaintenanceAccess/properties/label/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid4 = _errs15 === errors;
}
else {
var valid4 = true;
}
if(valid4){
if(data3.revoked !== undefined){
const _errs17 = errors;
if(typeof data3.revoked !== "boolean"){
validate155.errors = [{instancePath:instancePath+"/result/access/" + i0+"/revoked",schemaPath:"#/$defs/MaintenanceAccess/properties/revoked/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid4 = _errs17 === errors;
}
else {
var valid4 = true;
}
if(valid4){
if(data3.subject !== undefined){
const _errs19 = errors;
if(typeof data3.subject !== "string"){
validate155.errors = [{instancePath:instancePath+"/result/access/" + i0+"/subject",schemaPath:"#/$defs/MaintenanceAccess/properties/subject/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid4 = _errs19 === errors;
}
else {
var valid4 = true;
}
if(valid4){
if(data3.writable !== undefined){
const _errs21 = errors;
if(typeof data3.writable !== "boolean"){
validate155.errors = [{instancePath:instancePath+"/result/access/" + i0+"/writable",schemaPath:"#/$defs/MaintenanceAccess/properties/writable/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid4 = _errs21 === errors;
}
else {
var valid4 = true;
}
}
}
}
}
}
}
}
else {
validate155.errors = [{instancePath:instancePath+"/result/access/" + i0,schemaPath:"#/$defs/MaintenanceAccess/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
var valid2 = _errs9 === errors;
if(!valid2){
break;
}
}
}
else {
validate155.errors = [{instancePath:instancePath+"/result/access",schemaPath:"#/properties/result/properties/access/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs7 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.copies !== undefined){
let data9 = data1.copies;
const _errs23 = errors;
if(errors === _errs23){
if(Array.isArray(data9)){
var valid5 = true;
const len1 = data9.length;
for(let i1=0; i1<len1; i1++){
let data10 = data9[i1];
const _errs25 = errors;
const _errs26 = errors;
if(errors === _errs26){
if(data10 && typeof data10 == "object" && !Array.isArray(data10)){
let missing3;
if((((data10.id === undefined) && (missing3 = "id")) || ((data10.created_at_unix_ms === undefined) && (missing3 = "created_at_unix_ms"))) || ((data10.spaces === undefined) && (missing3 = "spaces"))){
validate155.errors = [{instancePath:instancePath+"/result/copies/" + i1,schemaPath:"#/$defs/FullCopy/required",keyword:"required",params:{missingProperty: missing3},message:"must have required property '"+missing3+"'"}];
return false;
}
else {
const _errs28 = errors;
for(const key3 in data10){
if(!(((key3 === "created_at_unix_ms") || (key3 === "id")) || (key3 === "spaces"))){
validate155.errors = [{instancePath:instancePath+"/result/copies/" + i1,schemaPath:"#/$defs/FullCopy/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key3},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs28 === errors){
if(data10.created_at_unix_ms !== undefined){
let data11 = data10.created_at_unix_ms;
const _errs29 = errors;
if(!((typeof data11 == "number") && (!(data11 % 1) && !isNaN(data11)))){
validate155.errors = [{instancePath:instancePath+"/result/copies/" + i1+"/created_at_unix_ms",schemaPath:"#/$defs/FullCopy/properties/created_at_unix_ms/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs29){
if(typeof data11 == "number"){
if(data11 < 0 || isNaN(data11)){
validate155.errors = [{instancePath:instancePath+"/result/copies/" + i1+"/created_at_unix_ms",schemaPath:"#/$defs/FullCopy/properties/created_at_unix_ms/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
var valid7 = _errs29 === errors;
}
else {
var valid7 = true;
}
if(valid7){
if(data10.id !== undefined){
const _errs31 = errors;
if(typeof data10.id !== "string"){
validate155.errors = [{instancePath:instancePath+"/result/copies/" + i1+"/id",schemaPath:"#/$defs/FullCopy/properties/id/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid7 = _errs31 === errors;
}
else {
var valid7 = true;
}
if(valid7){
if(data10.spaces !== undefined){
let data13 = data10.spaces;
const _errs33 = errors;
if(!((typeof data13 == "number") && (!(data13 % 1) && !isNaN(data13)))){
validate155.errors = [{instancePath:instancePath+"/result/copies/" + i1+"/spaces",schemaPath:"#/$defs/FullCopy/properties/spaces/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs33){
if(typeof data13 == "number"){
if(data13 < 0 || isNaN(data13)){
validate155.errors = [{instancePath:instancePath+"/result/copies/" + i1+"/spaces",schemaPath:"#/$defs/FullCopy/properties/spaces/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
var valid7 = _errs33 === errors;
}
else {
var valid7 = true;
}
}
}
}
}
}
else {
validate155.errors = [{instancePath:instancePath+"/result/copies/" + i1,schemaPath:"#/$defs/FullCopy/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
var valid5 = _errs25 === errors;
if(!valid5){
break;
}
}
}
else {
validate155.errors = [{instancePath:instancePath+"/result/copies",schemaPath:"#/properties/result/properties/copies/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs23 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.endpoint !== undefined){
let data14 = data1.endpoint;
const _errs35 = errors;
if((typeof data14 !== "string") && (data14 !== null)){
validate155.errors = [{instancePath:instancePath+"/result/endpoint",schemaPath:"#/properties/result/properties/endpoint/type",keyword:"type",params:{type: schema145.properties.result.properties.endpoint.type},message:"must be string,null"}];
return false;
}
var valid1 = _errs35 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.folder !== undefined){
let data15 = data1.folder;
const _errs37 = errors;
if((typeof data15 !== "string") && (data15 !== null)){
validate155.errors = [{instancePath:instancePath+"/result/folder",schemaPath:"#/properties/result/properties/folder/type",keyword:"type",params:{type: schema145.properties.result.properties.folder.type},message:"must be string,null"}];
return false;
}
var valid1 = _errs37 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.folder_available !== undefined){
const _errs39 = errors;
if(typeof data1.folder_available !== "boolean"){
validate155.errors = [{instancePath:instancePath+"/result/folder_available",schemaPath:"#/properties/result/properties/folder_available/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid1 = _errs39 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.issued_subject !== undefined){
let data17 = data1.issued_subject;
const _errs41 = errors;
if((typeof data17 !== "string") && (data17 !== null)){
validate155.errors = [{instancePath:instancePath+"/result/issued_subject",schemaPath:"#/properties/result/properties/issued_subject/type",keyword:"type",params:{type: schema145.properties.result.properties.issued_subject.type},message:"must be string,null"}];
return false;
}
var valid1 = _errs41 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.normal_ready !== undefined){
const _errs43 = errors;
if(typeof data1.normal_ready !== "boolean"){
validate155.errors = [{instancePath:instancePath+"/result/normal_ready",schemaPath:"#/properties/result/properties/normal_ready/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid1 = _errs43 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.owner !== undefined){
const _errs45 = errors;
if(typeof data1.owner !== "boolean"){
validate155.errors = [{instancePath:instancePath+"/result/owner",schemaPath:"#/properties/result/properties/owner/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid1 = _errs45 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.plan !== undefined){
let data20 = data1.plan;
const _errs47 = errors;
const _errs48 = errors;
let valid8 = false;
let passing0 = null;
const _errs49 = errors;
if(!(validate156(data20, {instancePath:instancePath+"/result/plan",parentData:data1,parentDataProperty:"plan",rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate156.errors : vErrors.concat(validate156.errors);
errors = vErrors.length;
}
var _valid0 = _errs49 === errors;
if(_valid0){
valid8 = true;
passing0 = 0;
}
const _errs50 = errors;
if(data20 !== null){
const err0 = {instancePath:instancePath+"/result/plan",schemaPath:"#/properties/result/properties/plan/oneOf/1/type",keyword:"type",params:{type: "null"},message:"must be null"};
if(vErrors === null){
vErrors = [err0];
}
else {
vErrors.push(err0);
}
errors++;
}
var _valid0 = _errs50 === errors;
if(_valid0 && valid8){
valid8 = false;
passing0 = [passing0, 1];
}
else {
if(_valid0){
valid8 = true;
passing0 = 1;
}
}
if(!valid8){
const err1 = {instancePath:instancePath+"/result/plan",schemaPath:"#/properties/result/properties/plan/oneOf",keyword:"oneOf",params:{passingSchemas: passing0},message:"must match exactly one schema in oneOf"};
if(vErrors === null){
vErrors = [err1];
}
else {
vErrors.push(err1);
}
errors++;
validate155.errors = vErrors;
return false;
}
else {
errors = _errs48;
if(vErrors !== null){
if(_errs48){
vErrors.length = _errs48;
}
else {
vErrors = null;
}
}
}
var valid1 = _errs47 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.problems !== undefined){
let data21 = data1.problems;
const _errs52 = errors;
if(errors === _errs52){
if(Array.isArray(data21)){
var valid9 = true;
const len2 = data21.length;
for(let i2=0; i2<len2; i2++){
const _errs54 = errors;
if(typeof data21[i2] !== "string"){
validate155.errors = [{instancePath:instancePath+"/result/problems/" + i2,schemaPath:"#/properties/result/properties/problems/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid9 = _errs54 === errors;
if(!valid9){
break;
}
}
}
else {
validate155.errors = [{instancePath:instancePath+"/result/problems",schemaPath:"#/properties/result/properties/problems/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs52 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.reason !== undefined){
let data23 = data1.reason;
const _errs56 = errors;
if((typeof data23 !== "string") && (data23 !== null)){
validate155.errors = [{instancePath:instancePath+"/result/reason",schemaPath:"#/properties/result/properties/reason/type",keyword:"type",params:{type: schema145.properties.result.properties.reason.type},message:"must be string,null"}];
return false;
}
var valid1 = _errs56 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.version !== undefined){
let data24 = data1.version;
const _errs58 = errors;
const _errs59 = errors;
if(errors === _errs59){
if(Array.isArray(data24)){
if(data24.length > 32){
validate155.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/maxItems",keyword:"maxItems",params:{limit: 32},message:"must NOT have more than 32 items"}];
return false;
}
else {
if(data24.length < 32){
validate155.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/minItems",keyword:"minItems",params:{limit: 32},message:"must NOT have fewer than 32 items"}];
return false;
}
else {
var valid11 = true;
const len3 = data24.length;
for(let i3=0; i3<len3; i3++){
let data25 = data24[i3];
const _errs61 = errors;
if(!((typeof data25 == "number") && (!(data25 % 1) && !isNaN(data25)))){
validate155.errors = [{instancePath:instancePath+"/result/version/" + i3,schemaPath:"#/$defs/StateVersion/items/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs61){
if(typeof data25 == "number"){
if(data25 > 255 || isNaN(data25)){
validate155.errors = [{instancePath:instancePath+"/result/version/" + i3,schemaPath:"#/$defs/StateVersion/items/maximum",keyword:"maximum",params:{comparison: "<=", limit: 255},message:"must be <= 255"}];
return false;
}
else {
if(data25 < 0 || isNaN(data25)){
validate155.errors = [{instancePath:instancePath+"/result/version/" + i3,schemaPath:"#/$defs/StateVersion/items/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
}
var valid11 = _errs61 === errors;
if(!valid11){
break;
}
}
}
}
}
else {
validate155.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs58 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.writable !== undefined){
const _errs63 = errors;
if(typeof data1.writable !== "boolean"){
validate155.errors = [{instancePath:instancePath+"/result/writable",schemaPath:"#/properties/result/properties/writable/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid1 = _errs63 === errors;
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
}
}
else {
validate155.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/type",keyword:"type",params:{type: "object"},message:"must be object"}];
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
let data27 = data.status;
const _errs65 = errors;
if(typeof data27 !== "string"){
validate155.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data27 === "ok")){
validate155.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/enum",keyword:"enum",params:{allowedValues: schema44.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs65 === errors;
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
validate155.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate155.errors = vErrors;
return errors === 0;
}
validate155.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate154(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate154.evaluated;
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
if(!(validate155(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate155.errors : vErrors.concat(validate155.errors);
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
validate154.errors = vErrors;
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
validate154.errors = vErrors;
evaluated0.props = props0;
return errors === 0;
}
validate154.evaluated = {"dynamicProps":true,"dynamicItems":false};

export const CommandResponse_PairingSnapshot = validate160;
const schema152 = {"oneOf":[{"$ref":"#/$defs/CommandSuccess_PairingSnapshot"},{"$ref":"#/$defs/CommandFailure"}],"description":"Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.\nA success confirms the original completion, not the current object state.\nConstruct success only AFTER the storage transaction commits. Cancellation,\na lost response or uncertain commit must never be converted to success."};
const schema153 = {"type":"object","required":["status","result","replayed"],"properties":{"replayed":{"type":"boolean"},"result":{"type":"object","required":["request","invitation","label","created_at_unix_ms","status","version"],"properties":{"created_at_unix_ms":{"type":"integer","format":"int64","maximum":253402300799999,"minimum":0},"invitation":{"type":"string"},"label":{"type":"string"},"request":{"type":"string"},"status":{"$ref":"#/$defs/PairingStatus"},"subject":{"type":["string","null"],"description":"Ordinary access subject for a decided connection, never a secret."},"version":{"$ref":"#/$defs/StateVersion"}},"additionalProperties":false},"status":{"$ref":"#/$defs/SuccessStatus"}},"additionalProperties":false};
const schema154 = {"type":"string","enum":["pending","cancelled","rejected","confirmed","unlinked"]};

function validate161(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate161.evaluated;
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
validate161.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((key0 === "replayed") || (key0 === "result")) || (key0 === "status"))){
validate161.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.replayed !== undefined){
const _errs2 = errors;
if(typeof data.replayed !== "boolean"){
validate161.errors = [{instancePath:instancePath+"/replayed",schemaPath:"#/properties/replayed/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
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
if(((((((data1.request === undefined) && (missing1 = "request")) || ((data1.invitation === undefined) && (missing1 = "invitation"))) || ((data1.label === undefined) && (missing1 = "label"))) || ((data1.created_at_unix_ms === undefined) && (missing1 = "created_at_unix_ms"))) || ((data1.status === undefined) && (missing1 = "status"))) || ((data1.version === undefined) && (missing1 = "version"))){
validate161.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/required",keyword:"required",params:{missingProperty: missing1},message:"must have required property '"+missing1+"'"}];
return false;
}
else {
const _errs6 = errors;
for(const key1 in data1){
if(!(((((((key1 === "created_at_unix_ms") || (key1 === "invitation")) || (key1 === "label")) || (key1 === "request")) || (key1 === "status")) || (key1 === "subject")) || (key1 === "version"))){
validate161.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key1},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs6 === errors){
if(data1.created_at_unix_ms !== undefined){
let data2 = data1.created_at_unix_ms;
const _errs7 = errors;
if(!((typeof data2 == "number") && (!(data2 % 1) && !isNaN(data2)))){
validate161.errors = [{instancePath:instancePath+"/result/created_at_unix_ms",schemaPath:"#/properties/result/properties/created_at_unix_ms/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs7){
if(typeof data2 == "number"){
if(data2 > 253402300799999 || isNaN(data2)){
validate161.errors = [{instancePath:instancePath+"/result/created_at_unix_ms",schemaPath:"#/properties/result/properties/created_at_unix_ms/maximum",keyword:"maximum",params:{comparison: "<=", limit: 253402300799999},message:"must be <= 253402300799999"}];
return false;
}
else {
if(data2 < 0 || isNaN(data2)){
validate161.errors = [{instancePath:instancePath+"/result/created_at_unix_ms",schemaPath:"#/properties/result/properties/created_at_unix_ms/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
}
var valid1 = _errs7 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.invitation !== undefined){
const _errs9 = errors;
if(typeof data1.invitation !== "string"){
validate161.errors = [{instancePath:instancePath+"/result/invitation",schemaPath:"#/properties/result/properties/invitation/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid1 = _errs9 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.label !== undefined){
const _errs11 = errors;
if(typeof data1.label !== "string"){
validate161.errors = [{instancePath:instancePath+"/result/label",schemaPath:"#/properties/result/properties/label/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid1 = _errs11 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.request !== undefined){
const _errs13 = errors;
if(typeof data1.request !== "string"){
validate161.errors = [{instancePath:instancePath+"/result/request",schemaPath:"#/properties/result/properties/request/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid1 = _errs13 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.status !== undefined){
let data6 = data1.status;
const _errs15 = errors;
if(typeof data6 !== "string"){
validate161.errors = [{instancePath:instancePath+"/result/status",schemaPath:"#/$defs/PairingStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(((((data6 === "pending") || (data6 === "cancelled")) || (data6 === "rejected")) || (data6 === "confirmed")) || (data6 === "unlinked"))){
validate161.errors = [{instancePath:instancePath+"/result/status",schemaPath:"#/$defs/PairingStatus/enum",keyword:"enum",params:{allowedValues: schema154.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid1 = _errs15 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.subject !== undefined){
let data7 = data1.subject;
const _errs18 = errors;
if((typeof data7 !== "string") && (data7 !== null)){
validate161.errors = [{instancePath:instancePath+"/result/subject",schemaPath:"#/properties/result/properties/subject/type",keyword:"type",params:{type: schema153.properties.result.properties.subject.type},message:"must be string,null"}];
return false;
}
var valid1 = _errs18 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.version !== undefined){
let data8 = data1.version;
const _errs20 = errors;
const _errs21 = errors;
if(errors === _errs21){
if(Array.isArray(data8)){
if(data8.length > 32){
validate161.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/maxItems",keyword:"maxItems",params:{limit: 32},message:"must NOT have more than 32 items"}];
return false;
}
else {
if(data8.length < 32){
validate161.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/minItems",keyword:"minItems",params:{limit: 32},message:"must NOT have fewer than 32 items"}];
return false;
}
else {
var valid4 = true;
const len0 = data8.length;
for(let i0=0; i0<len0; i0++){
let data9 = data8[i0];
const _errs23 = errors;
if(!((typeof data9 == "number") && (!(data9 % 1) && !isNaN(data9)))){
validate161.errors = [{instancePath:instancePath+"/result/version/" + i0,schemaPath:"#/$defs/StateVersion/items/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs23){
if(typeof data9 == "number"){
if(data9 > 255 || isNaN(data9)){
validate161.errors = [{instancePath:instancePath+"/result/version/" + i0,schemaPath:"#/$defs/StateVersion/items/maximum",keyword:"maximum",params:{comparison: "<=", limit: 255},message:"must be <= 255"}];
return false;
}
else {
if(data9 < 0 || isNaN(data9)){
validate161.errors = [{instancePath:instancePath+"/result/version/" + i0,schemaPath:"#/$defs/StateVersion/items/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
}
var valid4 = _errs23 === errors;
if(!valid4){
break;
}
}
}
}
}
else {
validate161.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs20 === errors;
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
else {
validate161.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/type",keyword:"type",params:{type: "object"},message:"must be object"}];
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
let data10 = data.status;
const _errs25 = errors;
if(typeof data10 !== "string"){
validate161.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data10 === "ok")){
validate161.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/enum",keyword:"enum",params:{allowedValues: schema44.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs25 === errors;
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
validate161.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate161.errors = vErrors;
return errors === 0;
}
validate161.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate160(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate160.evaluated;
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
if(!(validate161(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate161.errors : vErrors.concat(validate161.errors);
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
validate160.errors = vErrors;
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
validate160.errors = vErrors;
evaluated0.props = props0;
return errors === 0;
}
validate160.evaluated = {"dynamicProps":true,"dynamicItems":false};

export const CommandResponse_PairingList = validate164;
const schema157 = {"oneOf":[{"$ref":"#/$defs/CommandSuccess_PairingList"},{"$ref":"#/$defs/CommandFailure"}],"description":"Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.\nA success confirms the original completion, not the current object state.\nConstruct success only AFTER the storage transaction commits. Cancellation,\na lost response or uncertain commit must never be converted to success."};
const schema158 = {"type":"object","required":["status","result","replayed"],"properties":{"replayed":{"type":"boolean"},"result":{"type":"object","required":["entries"],"properties":{"entries":{"type":"array","items":{"$ref":"#/$defs/PairingSnapshot"},"maxItems":50},"next":{"type":["string","null"]}},"additionalProperties":false},"status":{"$ref":"#/$defs/SuccessStatus"}},"additionalProperties":false};
const schema159 = {"type":"object","required":["request","invitation","label","created_at_unix_ms","status","version"],"properties":{"created_at_unix_ms":{"type":"integer","format":"int64","maximum":253402300799999,"minimum":0},"invitation":{"type":"string"},"label":{"type":"string"},"request":{"type":"string"},"status":{"$ref":"#/$defs/PairingStatus"},"subject":{"type":["string","null"],"description":"Ordinary access subject for a decided connection, never a secret."},"version":{"$ref":"#/$defs/StateVersion"}},"additionalProperties":false};

function validate166(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate166.evaluated;
if(evaluated0.dynamicProps){
evaluated0.props = undefined;
}
if(evaluated0.dynamicItems){
evaluated0.items = undefined;
}
if(errors === 0){
if(data && typeof data == "object" && !Array.isArray(data)){
let missing0;
if(((((((data.request === undefined) && (missing0 = "request")) || ((data.invitation === undefined) && (missing0 = "invitation"))) || ((data.label === undefined) && (missing0 = "label"))) || ((data.created_at_unix_ms === undefined) && (missing0 = "created_at_unix_ms"))) || ((data.status === undefined) && (missing0 = "status"))) || ((data.version === undefined) && (missing0 = "version"))){
validate166.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((((((key0 === "created_at_unix_ms") || (key0 === "invitation")) || (key0 === "label")) || (key0 === "request")) || (key0 === "status")) || (key0 === "subject")) || (key0 === "version"))){
validate166.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.created_at_unix_ms !== undefined){
let data0 = data.created_at_unix_ms;
const _errs2 = errors;
if(!((typeof data0 == "number") && (!(data0 % 1) && !isNaN(data0)))){
validate166.errors = [{instancePath:instancePath+"/created_at_unix_ms",schemaPath:"#/properties/created_at_unix_ms/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs2){
if(typeof data0 == "number"){
if(data0 > 253402300799999 || isNaN(data0)){
validate166.errors = [{instancePath:instancePath+"/created_at_unix_ms",schemaPath:"#/properties/created_at_unix_ms/maximum",keyword:"maximum",params:{comparison: "<=", limit: 253402300799999},message:"must be <= 253402300799999"}];
return false;
}
else {
if(data0 < 0 || isNaN(data0)){
validate166.errors = [{instancePath:instancePath+"/created_at_unix_ms",schemaPath:"#/properties/created_at_unix_ms/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
}
var valid0 = _errs2 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.invitation !== undefined){
const _errs4 = errors;
if(typeof data.invitation !== "string"){
validate166.errors = [{instancePath:instancePath+"/invitation",schemaPath:"#/properties/invitation/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid0 = _errs4 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.label !== undefined){
const _errs6 = errors;
if(typeof data.label !== "string"){
validate166.errors = [{instancePath:instancePath+"/label",schemaPath:"#/properties/label/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid0 = _errs6 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.request !== undefined){
const _errs8 = errors;
if(typeof data.request !== "string"){
validate166.errors = [{instancePath:instancePath+"/request",schemaPath:"#/properties/request/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid0 = _errs8 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.status !== undefined){
let data4 = data.status;
const _errs10 = errors;
if(typeof data4 !== "string"){
validate166.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/PairingStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(((((data4 === "pending") || (data4 === "cancelled")) || (data4 === "rejected")) || (data4 === "confirmed")) || (data4 === "unlinked"))){
validate166.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/PairingStatus/enum",keyword:"enum",params:{allowedValues: schema154.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs10 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.subject !== undefined){
let data5 = data.subject;
const _errs13 = errors;
if((typeof data5 !== "string") && (data5 !== null)){
validate166.errors = [{instancePath:instancePath+"/subject",schemaPath:"#/properties/subject/type",keyword:"type",params:{type: schema159.properties.subject.type},message:"must be string,null"}];
return false;
}
var valid0 = _errs13 === errors;
}
else {
var valid0 = true;
}
if(valid0){
if(data.version !== undefined){
let data6 = data.version;
const _errs15 = errors;
const _errs16 = errors;
if(errors === _errs16){
if(Array.isArray(data6)){
if(data6.length > 32){
validate166.errors = [{instancePath:instancePath+"/version",schemaPath:"#/$defs/StateVersion/maxItems",keyword:"maxItems",params:{limit: 32},message:"must NOT have more than 32 items"}];
return false;
}
else {
if(data6.length < 32){
validate166.errors = [{instancePath:instancePath+"/version",schemaPath:"#/$defs/StateVersion/minItems",keyword:"minItems",params:{limit: 32},message:"must NOT have fewer than 32 items"}];
return false;
}
else {
var valid3 = true;
const len0 = data6.length;
for(let i0=0; i0<len0; i0++){
let data7 = data6[i0];
const _errs18 = errors;
if(!((typeof data7 == "number") && (!(data7 % 1) && !isNaN(data7)))){
validate166.errors = [{instancePath:instancePath+"/version/" + i0,schemaPath:"#/$defs/StateVersion/items/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs18){
if(typeof data7 == "number"){
if(data7 > 255 || isNaN(data7)){
validate166.errors = [{instancePath:instancePath+"/version/" + i0,schemaPath:"#/$defs/StateVersion/items/maximum",keyword:"maximum",params:{comparison: "<=", limit: 255},message:"must be <= 255"}];
return false;
}
else {
if(data7 < 0 || isNaN(data7)){
validate166.errors = [{instancePath:instancePath+"/version/" + i0,schemaPath:"#/$defs/StateVersion/items/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
}
var valid3 = _errs18 === errors;
if(!valid3){
break;
}
}
}
}
}
else {
validate166.errors = [{instancePath:instancePath+"/version",schemaPath:"#/$defs/StateVersion/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid0 = _errs15 === errors;
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
else {
validate166.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate166.errors = vErrors;
return errors === 0;
}
validate166.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate165(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate165.evaluated;
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
validate165.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((key0 === "replayed") || (key0 === "result")) || (key0 === "status"))){
validate165.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.replayed !== undefined){
const _errs2 = errors;
if(typeof data.replayed !== "boolean"){
validate165.errors = [{instancePath:instancePath+"/replayed",schemaPath:"#/properties/replayed/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
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
if((data1.entries === undefined) && (missing1 = "entries")){
validate165.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/required",keyword:"required",params:{missingProperty: missing1},message:"must have required property '"+missing1+"'"}];
return false;
}
else {
const _errs6 = errors;
for(const key1 in data1){
if(!((key1 === "entries") || (key1 === "next"))){
validate165.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key1},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs6 === errors){
if(data1.entries !== undefined){
let data2 = data1.entries;
const _errs7 = errors;
if(errors === _errs7){
if(Array.isArray(data2)){
if(data2.length > 50){
validate165.errors = [{instancePath:instancePath+"/result/entries",schemaPath:"#/properties/result/properties/entries/maxItems",keyword:"maxItems",params:{limit: 50},message:"must NOT have more than 50 items"}];
return false;
}
else {
var valid2 = true;
const len0 = data2.length;
for(let i0=0; i0<len0; i0++){
const _errs9 = errors;
if(!(validate166(data2[i0], {instancePath:instancePath+"/result/entries/" + i0,parentData:data2,parentDataProperty:i0,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate166.errors : vErrors.concat(validate166.errors);
errors = vErrors.length;
}
var valid2 = _errs9 === errors;
if(!valid2){
break;
}
}
}
}
else {
validate165.errors = [{instancePath:instancePath+"/result/entries",schemaPath:"#/properties/result/properties/entries/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs7 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.next !== undefined){
let data4 = data1.next;
const _errs10 = errors;
if((typeof data4 !== "string") && (data4 !== null)){
validate165.errors = [{instancePath:instancePath+"/result/next",schemaPath:"#/properties/result/properties/next/type",keyword:"type",params:{type: schema158.properties.result.properties.next.type},message:"must be string,null"}];
return false;
}
var valid1 = _errs10 === errors;
}
else {
var valid1 = true;
}
}
}
}
}
else {
validate165.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/type",keyword:"type",params:{type: "object"},message:"must be object"}];
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
let data5 = data.status;
const _errs12 = errors;
if(typeof data5 !== "string"){
validate165.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data5 === "ok")){
validate165.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/enum",keyword:"enum",params:{allowedValues: schema44.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs12 === errors;
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
validate165.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate165.errors = vErrors;
return errors === 0;
}
validate165.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate164(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate164.evaluated;
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
if(!(validate165(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate165.errors : vErrors.concat(validate165.errors);
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
validate164.errors = vErrors;
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
validate164.errors = vErrors;
evaluated0.props = props0;
return errors === 0;
}
validate164.evaluated = {"dynamicProps":true,"dynamicItems":false};

export const CommandResponse_DailyBackups = validate170;
const schema163 = {"oneOf":[{"$ref":"#/$defs/CommandSuccess_DailyBackups"},{"$ref":"#/$defs/CommandFailure"}],"description":"Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.\nA success confirms the original completion, not the current object state.\nConstruct success only AFTER the storage transaction commits. Cancellation,\na lost response or uncertain commit must never be converted to success."};
const schema164 = {"type":"object","required":["status","result","replayed"],"properties":{"replayed":{"type":"boolean"},"result":{"type":"object","required":["space","status","copies","cleanup_pending"],"properties":{"cleanup_pending":{"type":"boolean"},"copies":{"type":"array","items":{"$ref":"#/$defs/DailyCopy"}},"failed_sequence":{"type":["integer","null"],"format":"int64","minimum":0},"last_success":{"oneOf":[{"$ref":"#/$defs/DailyCopy"},{"type":"null"}]},"retry_after_ms":{"type":["integer","null"],"format":"int64","minimum":0},"space":{"type":"string"},"status":{"$ref":"#/$defs/DailyStatus"}},"additionalProperties":false},"status":{"$ref":"#/$defs/SuccessStatus"}},"additionalProperties":false};
const schema165 = {"type":"object","required":["id","created_at_unix_ms"],"properties":{"created_at_unix_ms":{"type":"integer","format":"int64","minimum":0},"id":{"type":"string"}},"additionalProperties":false};
const schema167 = {"type":"string","enum":["ready","saved","unchanged","daily_limit","waiting","unavailable","calendar_rollback","running"]};

function validate171(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate171.evaluated;
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
validate171.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((key0 === "replayed") || (key0 === "result")) || (key0 === "status"))){
validate171.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.replayed !== undefined){
const _errs2 = errors;
if(typeof data.replayed !== "boolean"){
validate171.errors = [{instancePath:instancePath+"/replayed",schemaPath:"#/properties/replayed/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
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
if(((((data1.space === undefined) && (missing1 = "space")) || ((data1.status === undefined) && (missing1 = "status"))) || ((data1.copies === undefined) && (missing1 = "copies"))) || ((data1.cleanup_pending === undefined) && (missing1 = "cleanup_pending"))){
validate171.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/required",keyword:"required",params:{missingProperty: missing1},message:"must have required property '"+missing1+"'"}];
return false;
}
else {
const _errs6 = errors;
for(const key1 in data1){
if(!(((((((key1 === "cleanup_pending") || (key1 === "copies")) || (key1 === "failed_sequence")) || (key1 === "last_success")) || (key1 === "retry_after_ms")) || (key1 === "space")) || (key1 === "status"))){
validate171.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key1},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs6 === errors){
if(data1.cleanup_pending !== undefined){
const _errs7 = errors;
if(typeof data1.cleanup_pending !== "boolean"){
validate171.errors = [{instancePath:instancePath+"/result/cleanup_pending",schemaPath:"#/properties/result/properties/cleanup_pending/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid1 = _errs7 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.copies !== undefined){
let data3 = data1.copies;
const _errs9 = errors;
if(errors === _errs9){
if(Array.isArray(data3)){
var valid2 = true;
const len0 = data3.length;
for(let i0=0; i0<len0; i0++){
let data4 = data3[i0];
const _errs11 = errors;
const _errs12 = errors;
if(errors === _errs12){
if(data4 && typeof data4 == "object" && !Array.isArray(data4)){
let missing2;
if(((data4.id === undefined) && (missing2 = "id")) || ((data4.created_at_unix_ms === undefined) && (missing2 = "created_at_unix_ms"))){
validate171.errors = [{instancePath:instancePath+"/result/copies/" + i0,schemaPath:"#/$defs/DailyCopy/required",keyword:"required",params:{missingProperty: missing2},message:"must have required property '"+missing2+"'"}];
return false;
}
else {
const _errs14 = errors;
for(const key2 in data4){
if(!((key2 === "created_at_unix_ms") || (key2 === "id"))){
validate171.errors = [{instancePath:instancePath+"/result/copies/" + i0,schemaPath:"#/$defs/DailyCopy/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key2},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs14 === errors){
if(data4.created_at_unix_ms !== undefined){
let data5 = data4.created_at_unix_ms;
const _errs15 = errors;
if(!((typeof data5 == "number") && (!(data5 % 1) && !isNaN(data5)))){
validate171.errors = [{instancePath:instancePath+"/result/copies/" + i0+"/created_at_unix_ms",schemaPath:"#/$defs/DailyCopy/properties/created_at_unix_ms/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs15){
if(typeof data5 == "number"){
if(data5 < 0 || isNaN(data5)){
validate171.errors = [{instancePath:instancePath+"/result/copies/" + i0+"/created_at_unix_ms",schemaPath:"#/$defs/DailyCopy/properties/created_at_unix_ms/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
var valid4 = _errs15 === errors;
}
else {
var valid4 = true;
}
if(valid4){
if(data4.id !== undefined){
const _errs17 = errors;
if(typeof data4.id !== "string"){
validate171.errors = [{instancePath:instancePath+"/result/copies/" + i0+"/id",schemaPath:"#/$defs/DailyCopy/properties/id/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid4 = _errs17 === errors;
}
else {
var valid4 = true;
}
}
}
}
}
else {
validate171.errors = [{instancePath:instancePath+"/result/copies/" + i0,schemaPath:"#/$defs/DailyCopy/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
var valid2 = _errs11 === errors;
if(!valid2){
break;
}
}
}
else {
validate171.errors = [{instancePath:instancePath+"/result/copies",schemaPath:"#/properties/result/properties/copies/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs9 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.failed_sequence !== undefined){
let data7 = data1.failed_sequence;
const _errs19 = errors;
if((!((typeof data7 == "number") && (!(data7 % 1) && !isNaN(data7)))) && (data7 !== null)){
validate171.errors = [{instancePath:instancePath+"/result/failed_sequence",schemaPath:"#/properties/result/properties/failed_sequence/type",keyword:"type",params:{type: schema164.properties.result.properties.failed_sequence.type},message:"must be integer,null"}];
return false;
}
if(errors === _errs19){
if(typeof data7 == "number"){
if(data7 < 0 || isNaN(data7)){
validate171.errors = [{instancePath:instancePath+"/result/failed_sequence",schemaPath:"#/properties/result/properties/failed_sequence/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
var valid1 = _errs19 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.last_success !== undefined){
let data8 = data1.last_success;
const _errs21 = errors;
const _errs22 = errors;
let valid5 = false;
let passing0 = null;
const _errs23 = errors;
const _errs24 = errors;
if(errors === _errs24){
if(data8 && typeof data8 == "object" && !Array.isArray(data8)){
let missing3;
if(((data8.id === undefined) && (missing3 = "id")) || ((data8.created_at_unix_ms === undefined) && (missing3 = "created_at_unix_ms"))){
const err0 = {instancePath:instancePath+"/result/last_success",schemaPath:"#/$defs/DailyCopy/required",keyword:"required",params:{missingProperty: missing3},message:"must have required property '"+missing3+"'"};
if(vErrors === null){
vErrors = [err0];
}
else {
vErrors.push(err0);
}
errors++;
}
else {
const _errs26 = errors;
for(const key3 in data8){
if(!((key3 === "created_at_unix_ms") || (key3 === "id"))){
const err1 = {instancePath:instancePath+"/result/last_success",schemaPath:"#/$defs/DailyCopy/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key3},message:"must NOT have additional properties"};
if(vErrors === null){
vErrors = [err1];
}
else {
vErrors.push(err1);
}
errors++;
break;
}
}
if(_errs26 === errors){
if(data8.created_at_unix_ms !== undefined){
let data9 = data8.created_at_unix_ms;
const _errs27 = errors;
if(!((typeof data9 == "number") && (!(data9 % 1) && !isNaN(data9)))){
const err2 = {instancePath:instancePath+"/result/last_success/created_at_unix_ms",schemaPath:"#/$defs/DailyCopy/properties/created_at_unix_ms/type",keyword:"type",params:{type: "integer"},message:"must be integer"};
if(vErrors === null){
vErrors = [err2];
}
else {
vErrors.push(err2);
}
errors++;
}
if(errors === _errs27){
if(typeof data9 == "number"){
if(data9 < 0 || isNaN(data9)){
const err3 = {instancePath:instancePath+"/result/last_success/created_at_unix_ms",schemaPath:"#/$defs/DailyCopy/properties/created_at_unix_ms/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"};
if(vErrors === null){
vErrors = [err3];
}
else {
vErrors.push(err3);
}
errors++;
}
}
}
var valid7 = _errs27 === errors;
}
else {
var valid7 = true;
}
if(valid7){
if(data8.id !== undefined){
const _errs29 = errors;
if(typeof data8.id !== "string"){
const err4 = {instancePath:instancePath+"/result/last_success/id",schemaPath:"#/$defs/DailyCopy/properties/id/type",keyword:"type",params:{type: "string"},message:"must be string"};
if(vErrors === null){
vErrors = [err4];
}
else {
vErrors.push(err4);
}
errors++;
}
var valid7 = _errs29 === errors;
}
else {
var valid7 = true;
}
}
}
}
}
else {
const err5 = {instancePath:instancePath+"/result/last_success",schemaPath:"#/$defs/DailyCopy/type",keyword:"type",params:{type: "object"},message:"must be object"};
if(vErrors === null){
vErrors = [err5];
}
else {
vErrors.push(err5);
}
errors++;
}
}
var _valid0 = _errs23 === errors;
if(_valid0){
valid5 = true;
passing0 = 0;
}
const _errs31 = errors;
if(data8 !== null){
const err6 = {instancePath:instancePath+"/result/last_success",schemaPath:"#/properties/result/properties/last_success/oneOf/1/type",keyword:"type",params:{type: "null"},message:"must be null"};
if(vErrors === null){
vErrors = [err6];
}
else {
vErrors.push(err6);
}
errors++;
}
var _valid0 = _errs31 === errors;
if(_valid0 && valid5){
valid5 = false;
passing0 = [passing0, 1];
}
else {
if(_valid0){
valid5 = true;
passing0 = 1;
}
}
if(!valid5){
const err7 = {instancePath:instancePath+"/result/last_success",schemaPath:"#/properties/result/properties/last_success/oneOf",keyword:"oneOf",params:{passingSchemas: passing0},message:"must match exactly one schema in oneOf"};
if(vErrors === null){
vErrors = [err7];
}
else {
vErrors.push(err7);
}
errors++;
validate171.errors = vErrors;
return false;
}
else {
errors = _errs22;
if(vErrors !== null){
if(_errs22){
vErrors.length = _errs22;
}
else {
vErrors = null;
}
}
}
var valid1 = _errs21 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.retry_after_ms !== undefined){
let data11 = data1.retry_after_ms;
const _errs33 = errors;
if((!((typeof data11 == "number") && (!(data11 % 1) && !isNaN(data11)))) && (data11 !== null)){
validate171.errors = [{instancePath:instancePath+"/result/retry_after_ms",schemaPath:"#/properties/result/properties/retry_after_ms/type",keyword:"type",params:{type: schema164.properties.result.properties.retry_after_ms.type},message:"must be integer,null"}];
return false;
}
if(errors === _errs33){
if(typeof data11 == "number"){
if(data11 < 0 || isNaN(data11)){
validate171.errors = [{instancePath:instancePath+"/result/retry_after_ms",schemaPath:"#/properties/result/properties/retry_after_ms/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
var valid1 = _errs33 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.space !== undefined){
const _errs35 = errors;
if(typeof data1.space !== "string"){
validate171.errors = [{instancePath:instancePath+"/result/space",schemaPath:"#/properties/result/properties/space/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid1 = _errs35 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.status !== undefined){
let data13 = data1.status;
const _errs37 = errors;
if(typeof data13 !== "string"){
validate171.errors = [{instancePath:instancePath+"/result/status",schemaPath:"#/$defs/DailyStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!((((((((data13 === "ready") || (data13 === "saved")) || (data13 === "unchanged")) || (data13 === "daily_limit")) || (data13 === "waiting")) || (data13 === "unavailable")) || (data13 === "calendar_rollback")) || (data13 === "running"))){
validate171.errors = [{instancePath:instancePath+"/result/status",schemaPath:"#/$defs/DailyStatus/enum",keyword:"enum",params:{allowedValues: schema167.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid1 = _errs37 === errors;
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
else {
validate171.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/type",keyword:"type",params:{type: "object"},message:"must be object"}];
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
const _errs40 = errors;
if(typeof data14 !== "string"){
validate171.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data14 === "ok")){
validate171.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/enum",keyword:"enum",params:{allowedValues: schema44.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs40 === errors;
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
validate171.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate171.errors = vErrors;
return errors === 0;
}
validate171.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate170(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate170.evaluated;
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
if(!(validate171(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate171.errors : vErrors.concat(validate171.errors);
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
validate170.errors = vErrors;
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
validate170.errors = vErrors;
evaluated0.props = props0;
return errors === 0;
}
validate170.evaluated = {"dynamicProps":true,"dynamicItems":false};

export const CommandResponse_SpaceSnapshot = validate174;
const schema169 = {"oneOf":[{"$ref":"#/$defs/CommandSuccess_SpaceSnapshot"},{"$ref":"#/$defs/CommandFailure"}],"description":"Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.\nA success confirms the original completion, not the current object state.\nConstruct success only AFTER the storage transaction commits. Cancellation,\na lost response or uncertain commit must never be converted to success."};
const schema170 = {"type":"object","required":["status","result","replayed"],"properties":{"replayed":{"type":"boolean"},"result":{"type":"object","required":["space","names","state","deletion_groups","empty","version"],"properties":{"contents":{"oneOf":[{"$ref":"#/$defs/SpaceContents","description":"Authoritative scope preview. None blocks lifecycle changes."},{"type":"null"}]},"deletion_groups":{"type":"array","items":{"type":"string"}},"empty":{"type":"boolean"},"names":{"type":"array","items":{"type":"string"},"description":"All distinct current names; multiple names require conflict resolution."},"space":{"type":"string"},"state":{"$ref":"#/$defs/SpaceState"},"version":{"$ref":"#/$defs/StateVersion"}},"additionalProperties":false},"status":{"$ref":"#/$defs/SuccessStatus"}},"additionalProperties":false};

function validate175(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate175.evaluated;
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
validate175.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((key0 === "replayed") || (key0 === "result")) || (key0 === "status"))){
validate175.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.replayed !== undefined){
const _errs2 = errors;
if(typeof data.replayed !== "boolean"){
validate175.errors = [{instancePath:instancePath+"/replayed",schemaPath:"#/properties/replayed/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
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
validate175.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/required",keyword:"required",params:{missingProperty: missing1},message:"must have required property '"+missing1+"'"}];
return false;
}
else {
const _errs6 = errors;
for(const key1 in data1){
if(!(((((((key1 === "contents") || (key1 === "deletion_groups")) || (key1 === "empty")) || (key1 === "names")) || (key1 === "space")) || (key1 === "state")) || (key1 === "version"))){
validate175.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key1},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs6 === errors){
if(data1.contents !== undefined){
let data2 = data1.contents;
const _errs7 = errors;
const _errs8 = errors;
let valid2 = false;
let passing0 = null;
const _errs9 = errors;
if(!(validate106(data2, {instancePath:instancePath+"/result/contents",parentData:data1,parentDataProperty:"contents",rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate106.errors : vErrors.concat(validate106.errors);
errors = vErrors.length;
}
var _valid0 = _errs9 === errors;
if(_valid0){
valid2 = true;
passing0 = 0;
}
const _errs10 = errors;
if(data2 !== null){
const err0 = {instancePath:instancePath+"/result/contents",schemaPath:"#/properties/result/properties/contents/oneOf/1/type",keyword:"type",params:{type: "null"},message:"must be null"};
if(vErrors === null){
vErrors = [err0];
}
else {
vErrors.push(err0);
}
errors++;
}
var _valid0 = _errs10 === errors;
if(_valid0 && valid2){
valid2 = false;
passing0 = [passing0, 1];
}
else {
if(_valid0){
valid2 = true;
passing0 = 1;
}
}
if(!valid2){
const err1 = {instancePath:instancePath+"/result/contents",schemaPath:"#/properties/result/properties/contents/oneOf",keyword:"oneOf",params:{passingSchemas: passing0},message:"must match exactly one schema in oneOf"};
if(vErrors === null){
vErrors = [err1];
}
else {
vErrors.push(err1);
}
errors++;
validate175.errors = vErrors;
return false;
}
else {
errors = _errs8;
if(vErrors !== null){
if(_errs8){
vErrors.length = _errs8;
}
else {
vErrors = null;
}
}
}
var valid1 = _errs7 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.deletion_groups !== undefined){
let data3 = data1.deletion_groups;
const _errs12 = errors;
if(errors === _errs12){
if(Array.isArray(data3)){
var valid3 = true;
const len0 = data3.length;
for(let i0=0; i0<len0; i0++){
const _errs14 = errors;
if(typeof data3[i0] !== "string"){
validate175.errors = [{instancePath:instancePath+"/result/deletion_groups/" + i0,schemaPath:"#/properties/result/properties/deletion_groups/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid3 = _errs14 === errors;
if(!valid3){
break;
}
}
}
else {
validate175.errors = [{instancePath:instancePath+"/result/deletion_groups",schemaPath:"#/properties/result/properties/deletion_groups/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs12 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.empty !== undefined){
const _errs16 = errors;
if(typeof data1.empty !== "boolean"){
validate175.errors = [{instancePath:instancePath+"/result/empty",schemaPath:"#/properties/result/properties/empty/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid1 = _errs16 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.names !== undefined){
let data6 = data1.names;
const _errs18 = errors;
if(errors === _errs18){
if(Array.isArray(data6)){
var valid4 = true;
const len1 = data6.length;
for(let i1=0; i1<len1; i1++){
const _errs20 = errors;
if(typeof data6[i1] !== "string"){
validate175.errors = [{instancePath:instancePath+"/result/names/" + i1,schemaPath:"#/properties/result/properties/names/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid4 = _errs20 === errors;
if(!valid4){
break;
}
}
}
else {
validate175.errors = [{instancePath:instancePath+"/result/names",schemaPath:"#/properties/result/properties/names/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs18 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.space !== undefined){
const _errs22 = errors;
if(typeof data1.space !== "string"){
validate175.errors = [{instancePath:instancePath+"/result/space",schemaPath:"#/properties/result/properties/space/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid1 = _errs22 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.state !== undefined){
let data9 = data1.state;
const _errs24 = errors;
if(typeof data9 !== "string"){
validate175.errors = [{instancePath:instancePath+"/result/state",schemaPath:"#/$defs/SpaceState/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(((data9 === "active") || (data9 === "trashed")) || (data9 === "conflicted"))){
validate175.errors = [{instancePath:instancePath+"/result/state",schemaPath:"#/$defs/SpaceState/enum",keyword:"enum",params:{allowedValues: schema78.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid1 = _errs24 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.version !== undefined){
let data10 = data1.version;
const _errs27 = errors;
const _errs28 = errors;
if(errors === _errs28){
if(Array.isArray(data10)){
if(data10.length > 32){
validate175.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/maxItems",keyword:"maxItems",params:{limit: 32},message:"must NOT have more than 32 items"}];
return false;
}
else {
if(data10.length < 32){
validate175.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/minItems",keyword:"minItems",params:{limit: 32},message:"must NOT have fewer than 32 items"}];
return false;
}
else {
var valid7 = true;
const len2 = data10.length;
for(let i2=0; i2<len2; i2++){
let data11 = data10[i2];
const _errs30 = errors;
if(!((typeof data11 == "number") && (!(data11 % 1) && !isNaN(data11)))){
validate175.errors = [{instancePath:instancePath+"/result/version/" + i2,schemaPath:"#/$defs/StateVersion/items/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs30){
if(typeof data11 == "number"){
if(data11 > 255 || isNaN(data11)){
validate175.errors = [{instancePath:instancePath+"/result/version/" + i2,schemaPath:"#/$defs/StateVersion/items/maximum",keyword:"maximum",params:{comparison: "<=", limit: 255},message:"must be <= 255"}];
return false;
}
else {
if(data11 < 0 || isNaN(data11)){
validate175.errors = [{instancePath:instancePath+"/result/version/" + i2,schemaPath:"#/$defs/StateVersion/items/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
}
var valid7 = _errs30 === errors;
if(!valid7){
break;
}
}
}
}
}
else {
validate175.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs27 === errors;
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
else {
validate175.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/type",keyword:"type",params:{type: "object"},message:"must be object"}];
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
let data12 = data.status;
const _errs32 = errors;
if(typeof data12 !== "string"){
validate175.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data12 === "ok")){
validate175.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/enum",keyword:"enum",params:{allowedValues: schema44.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs32 === errors;
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
validate175.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate175.errors = vErrors;
return errors === 0;
}
validate175.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate174(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate174.evaluated;
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
if(!(validate175(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate175.errors : vErrors.concat(validate175.errors);
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
validate174.errors = vErrors;
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
validate174.errors = vErrors;
evaluated0.props = props0;
return errors === 0;
}
validate174.evaluated = {"dynamicProps":true,"dynamicItems":false};

export const CommandResponse_ImportedSpace = validate179;
const schema174 = {"oneOf":[{"$ref":"#/$defs/CommandSuccess_ImportedSpace"},{"$ref":"#/$defs/CommandFailure"}],"description":"Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.\nA success confirms the original completion, not the current object state.\nConstruct success only AFTER the storage transaction commits. Cancellation,\na lost response or uncertain commit must never be converted to success."};
const schema175 = {"type":"object","required":["status","result","replayed"],"properties":{"replayed":{"type":"boolean"},"result":{"type":"object","required":["space"],"properties":{"space":{"type":"string"}},"additionalProperties":false},"status":{"$ref":"#/$defs/SuccessStatus"}},"additionalProperties":false};

function validate180(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate180.evaluated;
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
validate180.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((key0 === "replayed") || (key0 === "result")) || (key0 === "status"))){
validate180.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.replayed !== undefined){
const _errs2 = errors;
if(typeof data.replayed !== "boolean"){
validate180.errors = [{instancePath:instancePath+"/replayed",schemaPath:"#/properties/replayed/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
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
if((data1.space === undefined) && (missing1 = "space")){
validate180.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/required",keyword:"required",params:{missingProperty: missing1},message:"must have required property '"+missing1+"'"}];
return false;
}
else {
const _errs6 = errors;
for(const key1 in data1){
if(!(key1 === "space")){
validate180.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key1},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs6 === errors){
if(data1.space !== undefined){
if(typeof data1.space !== "string"){
validate180.errors = [{instancePath:instancePath+"/result/space",schemaPath:"#/properties/result/properties/space/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
}
}
}
}
else {
validate180.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/type",keyword:"type",params:{type: "object"},message:"must be object"}];
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
let data3 = data.status;
const _errs9 = errors;
if(typeof data3 !== "string"){
validate180.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data3 === "ok")){
validate180.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/enum",keyword:"enum",params:{allowedValues: schema44.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs9 === errors;
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
validate180.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate180.errors = vErrors;
return errors === 0;
}
validate180.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate179(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate179.evaluated;
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
if(!(validate180(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate180.errors : vErrors.concat(validate180.errors);
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
validate179.errors = vErrors;
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
validate179.errors = vErrors;
evaluated0.props = props0;
return errors === 0;
}
validate179.evaluated = {"dynamicProps":true,"dynamicItems":false};

export const CommandResponse_SpaceList = validate183;
const schema177 = {"oneOf":[{"$ref":"#/$defs/CommandSuccess_SpaceList"},{"$ref":"#/$defs/CommandFailure"}],"description":"Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.\nA success confirms the original completion, not the current object state.\nConstruct success only AFTER the storage transaction commits. Cancellation,\na lost response or uncertain commit must never be converted to success."};
const schema178 = {"type":"object","required":["status","result","replayed"],"properties":{"replayed":{"type":"boolean"},"result":{"type":"object","required":["spaces"],"properties":{"spaces":{"type":"array","items":{"$ref":"#/$defs/SpaceSnapshot"}}},"additionalProperties":false},"status":{"$ref":"#/$defs/SuccessStatus"}},"additionalProperties":false};

function validate184(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate184.evaluated;
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
validate184.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((key0 === "replayed") || (key0 === "result")) || (key0 === "status"))){
validate184.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.replayed !== undefined){
const _errs2 = errors;
if(typeof data.replayed !== "boolean"){
validate184.errors = [{instancePath:instancePath+"/replayed",schemaPath:"#/properties/replayed/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
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
validate184.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/required",keyword:"required",params:{missingProperty: missing1},message:"must have required property '"+missing1+"'"}];
return false;
}
else {
const _errs6 = errors;
for(const key1 in data1){
if(!(key1 === "spaces")){
validate184.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key1},message:"must NOT have additional properties"}];
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
if(!(validate105(data2[i0], {instancePath:instancePath+"/result/spaces/" + i0,parentData:data2,parentDataProperty:i0,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate105.errors : vErrors.concat(validate105.errors);
errors = vErrors.length;
}
var valid2 = _errs9 === errors;
if(!valid2){
break;
}
}
}
else {
validate184.errors = [{instancePath:instancePath+"/result/spaces",schemaPath:"#/properties/result/properties/spaces/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
}
}
}
}
else {
validate184.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/type",keyword:"type",params:{type: "object"},message:"must be object"}];
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
validate184.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data4 === "ok")){
validate184.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/enum",keyword:"enum",params:{allowedValues: schema44.enum},message:"must be equal to one of the allowed values"}];
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
validate184.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate184.errors = vErrors;
return errors === 0;
}
validate184.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate183(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate183.evaluated;
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
if(!(validate184(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate184.errors : vErrors.concat(validate184.errors);
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
validate183.errors = vErrors;
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
validate183.errors = vErrors;
evaluated0.props = props0;
return errors === 0;
}
validate183.evaluated = {"dynamicProps":true,"dynamicItems":false};

export const CommandResponse_RecoveryState = validate188;
const schema180 = {"oneOf":[{"$ref":"#/$defs/CommandSuccess_RecoveryState"},{"$ref":"#/$defs/CommandFailure"}],"description":"Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.\nA success confirms the original completion, not the current object state.\nConstruct success only AFTER the storage transaction commits. Cancellation,\na lost response or uncertain commit must never be converted to success."};
const schema181 = {"type":"object","required":["status","result","replayed"],"properties":{"replayed":{"type":"boolean"},"result":{"type":"object","required":["space","version","paused","pause_version","cleanup_pending"],"properties":{"backup":{"oneOf":[{"$ref":"#/$defs/SafetyBackup"},{"type":"null"}]},"cleanup_pending":{"type":"boolean"},"pause_version":{"$ref":"#/$defs/StateVersion"},"paused":{"type":"boolean"},"space":{"type":"string"},"version":{"$ref":"#/$defs/StateVersion"}},"additionalProperties":false},"status":{"$ref":"#/$defs/SuccessStatus"}},"additionalProperties":false};
const schema182 = {"type":"object","required":["id","created_at_unix_ms"],"properties":{"created_at_unix_ms":{"type":"integer","format":"int64","minimum":0},"id":{"type":"string"}},"additionalProperties":false};

function validate189(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate189.evaluated;
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
validate189.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((key0 === "replayed") || (key0 === "result")) || (key0 === "status"))){
validate189.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.replayed !== undefined){
const _errs2 = errors;
if(typeof data.replayed !== "boolean"){
validate189.errors = [{instancePath:instancePath+"/replayed",schemaPath:"#/properties/replayed/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
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
if((((((data1.space === undefined) && (missing1 = "space")) || ((data1.version === undefined) && (missing1 = "version"))) || ((data1.paused === undefined) && (missing1 = "paused"))) || ((data1.pause_version === undefined) && (missing1 = "pause_version"))) || ((data1.cleanup_pending === undefined) && (missing1 = "cleanup_pending"))){
validate189.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/required",keyword:"required",params:{missingProperty: missing1},message:"must have required property '"+missing1+"'"}];
return false;
}
else {
const _errs6 = errors;
for(const key1 in data1){
if(!((((((key1 === "backup") || (key1 === "cleanup_pending")) || (key1 === "pause_version")) || (key1 === "paused")) || (key1 === "space")) || (key1 === "version"))){
validate189.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key1},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs6 === errors){
if(data1.backup !== undefined){
let data2 = data1.backup;
const _errs7 = errors;
const _errs8 = errors;
let valid2 = false;
let passing0 = null;
const _errs9 = errors;
const _errs10 = errors;
if(errors === _errs10){
if(data2 && typeof data2 == "object" && !Array.isArray(data2)){
let missing2;
if(((data2.id === undefined) && (missing2 = "id")) || ((data2.created_at_unix_ms === undefined) && (missing2 = "created_at_unix_ms"))){
const err0 = {instancePath:instancePath+"/result/backup",schemaPath:"#/$defs/SafetyBackup/required",keyword:"required",params:{missingProperty: missing2},message:"must have required property '"+missing2+"'"};
if(vErrors === null){
vErrors = [err0];
}
else {
vErrors.push(err0);
}
errors++;
}
else {
const _errs12 = errors;
for(const key2 in data2){
if(!((key2 === "created_at_unix_ms") || (key2 === "id"))){
const err1 = {instancePath:instancePath+"/result/backup",schemaPath:"#/$defs/SafetyBackup/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key2},message:"must NOT have additional properties"};
if(vErrors === null){
vErrors = [err1];
}
else {
vErrors.push(err1);
}
errors++;
break;
}
}
if(_errs12 === errors){
if(data2.created_at_unix_ms !== undefined){
let data3 = data2.created_at_unix_ms;
const _errs13 = errors;
if(!((typeof data3 == "number") && (!(data3 % 1) && !isNaN(data3)))){
const err2 = {instancePath:instancePath+"/result/backup/created_at_unix_ms",schemaPath:"#/$defs/SafetyBackup/properties/created_at_unix_ms/type",keyword:"type",params:{type: "integer"},message:"must be integer"};
if(vErrors === null){
vErrors = [err2];
}
else {
vErrors.push(err2);
}
errors++;
}
if(errors === _errs13){
if(typeof data3 == "number"){
if(data3 < 0 || isNaN(data3)){
const err3 = {instancePath:instancePath+"/result/backup/created_at_unix_ms",schemaPath:"#/$defs/SafetyBackup/properties/created_at_unix_ms/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"};
if(vErrors === null){
vErrors = [err3];
}
else {
vErrors.push(err3);
}
errors++;
}
}
}
var valid4 = _errs13 === errors;
}
else {
var valid4 = true;
}
if(valid4){
if(data2.id !== undefined){
const _errs15 = errors;
if(typeof data2.id !== "string"){
const err4 = {instancePath:instancePath+"/result/backup/id",schemaPath:"#/$defs/SafetyBackup/properties/id/type",keyword:"type",params:{type: "string"},message:"must be string"};
if(vErrors === null){
vErrors = [err4];
}
else {
vErrors.push(err4);
}
errors++;
}
var valid4 = _errs15 === errors;
}
else {
var valid4 = true;
}
}
}
}
}
else {
const err5 = {instancePath:instancePath+"/result/backup",schemaPath:"#/$defs/SafetyBackup/type",keyword:"type",params:{type: "object"},message:"must be object"};
if(vErrors === null){
vErrors = [err5];
}
else {
vErrors.push(err5);
}
errors++;
}
}
var _valid0 = _errs9 === errors;
if(_valid0){
valid2 = true;
passing0 = 0;
}
const _errs17 = errors;
if(data2 !== null){
const err6 = {instancePath:instancePath+"/result/backup",schemaPath:"#/properties/result/properties/backup/oneOf/1/type",keyword:"type",params:{type: "null"},message:"must be null"};
if(vErrors === null){
vErrors = [err6];
}
else {
vErrors.push(err6);
}
errors++;
}
var _valid0 = _errs17 === errors;
if(_valid0 && valid2){
valid2 = false;
passing0 = [passing0, 1];
}
else {
if(_valid0){
valid2 = true;
passing0 = 1;
}
}
if(!valid2){
const err7 = {instancePath:instancePath+"/result/backup",schemaPath:"#/properties/result/properties/backup/oneOf",keyword:"oneOf",params:{passingSchemas: passing0},message:"must match exactly one schema in oneOf"};
if(vErrors === null){
vErrors = [err7];
}
else {
vErrors.push(err7);
}
errors++;
validate189.errors = vErrors;
return false;
}
else {
errors = _errs8;
if(vErrors !== null){
if(_errs8){
vErrors.length = _errs8;
}
else {
vErrors = null;
}
}
}
var valid1 = _errs7 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.cleanup_pending !== undefined){
const _errs19 = errors;
if(typeof data1.cleanup_pending !== "boolean"){
validate189.errors = [{instancePath:instancePath+"/result/cleanup_pending",schemaPath:"#/properties/result/properties/cleanup_pending/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid1 = _errs19 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.pause_version !== undefined){
let data6 = data1.pause_version;
const _errs21 = errors;
const _errs22 = errors;
if(errors === _errs22){
if(Array.isArray(data6)){
if(data6.length > 32){
validate189.errors = [{instancePath:instancePath+"/result/pause_version",schemaPath:"#/$defs/StateVersion/maxItems",keyword:"maxItems",params:{limit: 32},message:"must NOT have more than 32 items"}];
return false;
}
else {
if(data6.length < 32){
validate189.errors = [{instancePath:instancePath+"/result/pause_version",schemaPath:"#/$defs/StateVersion/minItems",keyword:"minItems",params:{limit: 32},message:"must NOT have fewer than 32 items"}];
return false;
}
else {
var valid6 = true;
const len0 = data6.length;
for(let i0=0; i0<len0; i0++){
let data7 = data6[i0];
const _errs24 = errors;
if(!((typeof data7 == "number") && (!(data7 % 1) && !isNaN(data7)))){
validate189.errors = [{instancePath:instancePath+"/result/pause_version/" + i0,schemaPath:"#/$defs/StateVersion/items/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs24){
if(typeof data7 == "number"){
if(data7 > 255 || isNaN(data7)){
validate189.errors = [{instancePath:instancePath+"/result/pause_version/" + i0,schemaPath:"#/$defs/StateVersion/items/maximum",keyword:"maximum",params:{comparison: "<=", limit: 255},message:"must be <= 255"}];
return false;
}
else {
if(data7 < 0 || isNaN(data7)){
validate189.errors = [{instancePath:instancePath+"/result/pause_version/" + i0,schemaPath:"#/$defs/StateVersion/items/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
}
var valid6 = _errs24 === errors;
if(!valid6){
break;
}
}
}
}
}
else {
validate189.errors = [{instancePath:instancePath+"/result/pause_version",schemaPath:"#/$defs/StateVersion/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs21 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.paused !== undefined){
const _errs26 = errors;
if(typeof data1.paused !== "boolean"){
validate189.errors = [{instancePath:instancePath+"/result/paused",schemaPath:"#/properties/result/properties/paused/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid1 = _errs26 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.space !== undefined){
const _errs28 = errors;
if(typeof data1.space !== "string"){
validate189.errors = [{instancePath:instancePath+"/result/space",schemaPath:"#/properties/result/properties/space/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid1 = _errs28 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.version !== undefined){
let data10 = data1.version;
const _errs30 = errors;
const _errs31 = errors;
if(errors === _errs31){
if(Array.isArray(data10)){
if(data10.length > 32){
validate189.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/maxItems",keyword:"maxItems",params:{limit: 32},message:"must NOT have more than 32 items"}];
return false;
}
else {
if(data10.length < 32){
validate189.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/minItems",keyword:"minItems",params:{limit: 32},message:"must NOT have fewer than 32 items"}];
return false;
}
else {
var valid8 = true;
const len1 = data10.length;
for(let i1=0; i1<len1; i1++){
let data11 = data10[i1];
const _errs33 = errors;
if(!((typeof data11 == "number") && (!(data11 % 1) && !isNaN(data11)))){
validate189.errors = [{instancePath:instancePath+"/result/version/" + i1,schemaPath:"#/$defs/StateVersion/items/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs33){
if(typeof data11 == "number"){
if(data11 > 255 || isNaN(data11)){
validate189.errors = [{instancePath:instancePath+"/result/version/" + i1,schemaPath:"#/$defs/StateVersion/items/maximum",keyword:"maximum",params:{comparison: "<=", limit: 255},message:"must be <= 255"}];
return false;
}
else {
if(data11 < 0 || isNaN(data11)){
validate189.errors = [{instancePath:instancePath+"/result/version/" + i1,schemaPath:"#/$defs/StateVersion/items/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
}
var valid8 = _errs33 === errors;
if(!valid8){
break;
}
}
}
}
}
else {
validate189.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs30 === errors;
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
validate189.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/type",keyword:"type",params:{type: "object"},message:"must be object"}];
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
let data12 = data.status;
const _errs35 = errors;
if(typeof data12 !== "string"){
validate189.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data12 === "ok")){
validate189.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/enum",keyword:"enum",params:{allowedValues: schema44.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs35 === errors;
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
validate189.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate189.errors = vErrors;
return errors === 0;
}
validate189.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate188(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate188.evaluated;
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
if(!(validate189(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate189.errors : vErrors.concat(validate189.errors);
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
validate188.errors = vErrors;
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
validate188.errors = vErrors;
evaluated0.props = props0;
return errors === 0;
}
validate188.evaluated = {"dynamicProps":true,"dynamicItems":false};

export const CommandResponse_MissingRecoveryState = validate192;
const schema186 = {"oneOf":[{"$ref":"#/$defs/CommandSuccess_MissingRecoveryState"},{"$ref":"#/$defs/CommandFailure"}],"description":"Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.\nA success confirms the original completion, not the current object state.\nConstruct success only AFTER the storage transaction commits. Cancellation,\na lost response or uncertain commit must never be converted to success."};
const schema187 = {"type":"object","required":["status","result","replayed"],"properties":{"replayed":{"type":"boolean"},"result":{"type":"object","required":["space","names","version"],"properties":{"names":{"type":"array","items":{"type":"string"}},"space":{"type":"string"},"version":{"$ref":"#/$defs/StateVersion"}},"additionalProperties":false},"status":{"$ref":"#/$defs/SuccessStatus"}},"additionalProperties":false};

function validate193(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate193.evaluated;
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
validate193.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((key0 === "replayed") || (key0 === "result")) || (key0 === "status"))){
validate193.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.replayed !== undefined){
const _errs2 = errors;
if(typeof data.replayed !== "boolean"){
validate193.errors = [{instancePath:instancePath+"/replayed",schemaPath:"#/properties/replayed/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
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
if((((data1.space === undefined) && (missing1 = "space")) || ((data1.names === undefined) && (missing1 = "names"))) || ((data1.version === undefined) && (missing1 = "version"))){
validate193.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/required",keyword:"required",params:{missingProperty: missing1},message:"must have required property '"+missing1+"'"}];
return false;
}
else {
const _errs6 = errors;
for(const key1 in data1){
if(!(((key1 === "names") || (key1 === "space")) || (key1 === "version"))){
validate193.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key1},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs6 === errors){
if(data1.names !== undefined){
let data2 = data1.names;
const _errs7 = errors;
if(errors === _errs7){
if(Array.isArray(data2)){
var valid2 = true;
const len0 = data2.length;
for(let i0=0; i0<len0; i0++){
const _errs9 = errors;
if(typeof data2[i0] !== "string"){
validate193.errors = [{instancePath:instancePath+"/result/names/" + i0,schemaPath:"#/properties/result/properties/names/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid2 = _errs9 === errors;
if(!valid2){
break;
}
}
}
else {
validate193.errors = [{instancePath:instancePath+"/result/names",schemaPath:"#/properties/result/properties/names/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs7 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.space !== undefined){
const _errs11 = errors;
if(typeof data1.space !== "string"){
validate193.errors = [{instancePath:instancePath+"/result/space",schemaPath:"#/properties/result/properties/space/type",keyword:"type",params:{type: "string"},message:"must be string"}];
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
validate193.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/maxItems",keyword:"maxItems",params:{limit: 32},message:"must NOT have more than 32 items"}];
return false;
}
else {
if(data5.length < 32){
validate193.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/minItems",keyword:"minItems",params:{limit: 32},message:"must NOT have fewer than 32 items"}];
return false;
}
else {
var valid4 = true;
const len1 = data5.length;
for(let i1=0; i1<len1; i1++){
let data6 = data5[i1];
const _errs16 = errors;
if(!((typeof data6 == "number") && (!(data6 % 1) && !isNaN(data6)))){
validate193.errors = [{instancePath:instancePath+"/result/version/" + i1,schemaPath:"#/$defs/StateVersion/items/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs16){
if(typeof data6 == "number"){
if(data6 > 255 || isNaN(data6)){
validate193.errors = [{instancePath:instancePath+"/result/version/" + i1,schemaPath:"#/$defs/StateVersion/items/maximum",keyword:"maximum",params:{comparison: "<=", limit: 255},message:"must be <= 255"}];
return false;
}
else {
if(data6 < 0 || isNaN(data6)){
validate193.errors = [{instancePath:instancePath+"/result/version/" + i1,schemaPath:"#/$defs/StateVersion/items/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
}
var valid4 = _errs16 === errors;
if(!valid4){
break;
}
}
}
}
}
else {
validate193.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs13 === errors;
}
else {
var valid1 = true;
}
}
}
}
}
}
else {
validate193.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/type",keyword:"type",params:{type: "object"},message:"must be object"}];
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
let data7 = data.status;
const _errs18 = errors;
if(typeof data7 !== "string"){
validate193.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data7 === "ok")){
validate193.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/enum",keyword:"enum",params:{allowedValues: schema44.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs18 === errors;
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
validate193.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate193.errors = vErrors;
return errors === 0;
}
validate193.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate192(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate192.evaluated;
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
if(!(validate193(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate193.errors : vErrors.concat(validate193.errors);
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
validate192.errors = vErrors;
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
validate192.errors = vErrors;
evaluated0.props = props0;
return errors === 0;
}
validate192.evaluated = {"dynamicProps":true,"dynamicItems":false};

export const CommandResponse_RestoredSnapshot = validate196;
const schema190 = {"oneOf":[{"$ref":"#/$defs/CommandSuccess_RestoredSnapshot"},{"$ref":"#/$defs/CommandFailure"}],"description":"Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.\nA success confirms the original completion, not the current object state.\nConstruct success only AFTER the storage transaction commits. Cancellation,\na lost response or uncertain commit must never be converted to success."};
const schema191 = {"type":"object","required":["status","result","replayed"],"properties":{"replayed":{"type":"boolean"},"result":{"type":"object","required":["space","safety"],"properties":{"safety":{"type":"string"},"space":{"type":"string"}},"additionalProperties":false},"status":{"$ref":"#/$defs/SuccessStatus"}},"additionalProperties":false};

function validate197(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate197.evaluated;
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
validate197.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((key0 === "replayed") || (key0 === "result")) || (key0 === "status"))){
validate197.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.replayed !== undefined){
const _errs2 = errors;
if(typeof data.replayed !== "boolean"){
validate197.errors = [{instancePath:instancePath+"/replayed",schemaPath:"#/properties/replayed/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
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
if(((data1.space === undefined) && (missing1 = "space")) || ((data1.safety === undefined) && (missing1 = "safety"))){
validate197.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/required",keyword:"required",params:{missingProperty: missing1},message:"must have required property '"+missing1+"'"}];
return false;
}
else {
const _errs6 = errors;
for(const key1 in data1){
if(!((key1 === "safety") || (key1 === "space"))){
validate197.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key1},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs6 === errors){
if(data1.safety !== undefined){
const _errs7 = errors;
if(typeof data1.safety !== "string"){
validate197.errors = [{instancePath:instancePath+"/result/safety",schemaPath:"#/properties/result/properties/safety/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid1 = _errs7 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.space !== undefined){
const _errs9 = errors;
if(typeof data1.space !== "string"){
validate197.errors = [{instancePath:instancePath+"/result/space",schemaPath:"#/properties/result/properties/space/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid1 = _errs9 === errors;
}
else {
var valid1 = true;
}
}
}
}
}
else {
validate197.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/type",keyword:"type",params:{type: "object"},message:"must be object"}];
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
const _errs11 = errors;
if(typeof data4 !== "string"){
validate197.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data4 === "ok")){
validate197.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/enum",keyword:"enum",params:{allowedValues: schema44.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs11 === errors;
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
validate197.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate197.errors = vErrors;
return errors === 0;
}
validate197.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate196(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate196.evaluated;
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
if(!(validate197(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate197.errors : vErrors.concat(validate197.errors);
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
validate196.errors = vErrors;
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
validate196.errors = vErrors;
evaluated0.props = props0;
return errors === 0;
}
validate196.evaluated = {"dynamicProps":true,"dynamicItems":false};

export const CommandResponse_SyncCatalog = validate200;
const schema193 = {"oneOf":[{"$ref":"#/$defs/CommandSuccess_SyncCatalog"},{"$ref":"#/$defs/CommandFailure"}],"description":"Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.\nA success confirms the original completion, not the current object state.\nConstruct success only AFTER the storage transaction commits. Cancellation,\na lost response or uncertain commit must never be converted to success."};
const schema194 = {"type":"object","required":["status","result","replayed"],"properties":{"replayed":{"type":"boolean"},"result":{"type":"object","required":["protocol","format","pairing","spaces"],"properties":{"format":{"type":"integer","format":"int32","minimum":0},"pairing":{"type":"string"},"protocol":{"type":"integer","format":"int32","minimum":0},"spaces":{"type":"array","items":{"$ref":"#/$defs/SyncEntry"}}},"additionalProperties":false},"status":{"$ref":"#/$defs/SuccessStatus"}},"additionalProperties":false};
const schema195 = {"type":"object","required":["space","names","writable","selected","paused"],"properties":{"names":{"type":"array","items":{"type":"string"}},"paused":{"type":"boolean"},"selected":{"type":"boolean"},"space":{"type":"string"},"writable":{"type":"boolean"}},"additionalProperties":false};

function validate201(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate201.evaluated;
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
validate201.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((key0 === "replayed") || (key0 === "result")) || (key0 === "status"))){
validate201.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.replayed !== undefined){
const _errs2 = errors;
if(typeof data.replayed !== "boolean"){
validate201.errors = [{instancePath:instancePath+"/replayed",schemaPath:"#/properties/replayed/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
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
if(((((data1.protocol === undefined) && (missing1 = "protocol")) || ((data1.format === undefined) && (missing1 = "format"))) || ((data1.pairing === undefined) && (missing1 = "pairing"))) || ((data1.spaces === undefined) && (missing1 = "spaces"))){
validate201.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/required",keyword:"required",params:{missingProperty: missing1},message:"must have required property '"+missing1+"'"}];
return false;
}
else {
const _errs6 = errors;
for(const key1 in data1){
if(!((((key1 === "format") || (key1 === "pairing")) || (key1 === "protocol")) || (key1 === "spaces"))){
validate201.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key1},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs6 === errors){
if(data1.format !== undefined){
let data2 = data1.format;
const _errs7 = errors;
if(!((typeof data2 == "number") && (!(data2 % 1) && !isNaN(data2)))){
validate201.errors = [{instancePath:instancePath+"/result/format",schemaPath:"#/properties/result/properties/format/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs7){
if(typeof data2 == "number"){
if(data2 < 0 || isNaN(data2)){
validate201.errors = [{instancePath:instancePath+"/result/format",schemaPath:"#/properties/result/properties/format/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
var valid1 = _errs7 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.pairing !== undefined){
const _errs9 = errors;
if(typeof data1.pairing !== "string"){
validate201.errors = [{instancePath:instancePath+"/result/pairing",schemaPath:"#/properties/result/properties/pairing/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid1 = _errs9 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.protocol !== undefined){
let data4 = data1.protocol;
const _errs11 = errors;
if(!((typeof data4 == "number") && (!(data4 % 1) && !isNaN(data4)))){
validate201.errors = [{instancePath:instancePath+"/result/protocol",schemaPath:"#/properties/result/properties/protocol/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs11){
if(typeof data4 == "number"){
if(data4 < 0 || isNaN(data4)){
validate201.errors = [{instancePath:instancePath+"/result/protocol",schemaPath:"#/properties/result/properties/protocol/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
var valid1 = _errs11 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.spaces !== undefined){
let data5 = data1.spaces;
const _errs13 = errors;
if(errors === _errs13){
if(Array.isArray(data5)){
var valid2 = true;
const len0 = data5.length;
for(let i0=0; i0<len0; i0++){
let data6 = data5[i0];
const _errs15 = errors;
const _errs16 = errors;
if(errors === _errs16){
if(data6 && typeof data6 == "object" && !Array.isArray(data6)){
let missing2;
if((((((data6.space === undefined) && (missing2 = "space")) || ((data6.names === undefined) && (missing2 = "names"))) || ((data6.writable === undefined) && (missing2 = "writable"))) || ((data6.selected === undefined) && (missing2 = "selected"))) || ((data6.paused === undefined) && (missing2 = "paused"))){
validate201.errors = [{instancePath:instancePath+"/result/spaces/" + i0,schemaPath:"#/$defs/SyncEntry/required",keyword:"required",params:{missingProperty: missing2},message:"must have required property '"+missing2+"'"}];
return false;
}
else {
const _errs18 = errors;
for(const key2 in data6){
if(!(((((key2 === "names") || (key2 === "paused")) || (key2 === "selected")) || (key2 === "space")) || (key2 === "writable"))){
validate201.errors = [{instancePath:instancePath+"/result/spaces/" + i0,schemaPath:"#/$defs/SyncEntry/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key2},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs18 === errors){
if(data6.names !== undefined){
let data7 = data6.names;
const _errs19 = errors;
if(errors === _errs19){
if(Array.isArray(data7)){
var valid5 = true;
const len1 = data7.length;
for(let i1=0; i1<len1; i1++){
const _errs21 = errors;
if(typeof data7[i1] !== "string"){
validate201.errors = [{instancePath:instancePath+"/result/spaces/" + i0+"/names/" + i1,schemaPath:"#/$defs/SyncEntry/properties/names/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid5 = _errs21 === errors;
if(!valid5){
break;
}
}
}
else {
validate201.errors = [{instancePath:instancePath+"/result/spaces/" + i0+"/names",schemaPath:"#/$defs/SyncEntry/properties/names/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid4 = _errs19 === errors;
}
else {
var valid4 = true;
}
if(valid4){
if(data6.paused !== undefined){
const _errs23 = errors;
if(typeof data6.paused !== "boolean"){
validate201.errors = [{instancePath:instancePath+"/result/spaces/" + i0+"/paused",schemaPath:"#/$defs/SyncEntry/properties/paused/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid4 = _errs23 === errors;
}
else {
var valid4 = true;
}
if(valid4){
if(data6.selected !== undefined){
const _errs25 = errors;
if(typeof data6.selected !== "boolean"){
validate201.errors = [{instancePath:instancePath+"/result/spaces/" + i0+"/selected",schemaPath:"#/$defs/SyncEntry/properties/selected/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid4 = _errs25 === errors;
}
else {
var valid4 = true;
}
if(valid4){
if(data6.space !== undefined){
const _errs27 = errors;
if(typeof data6.space !== "string"){
validate201.errors = [{instancePath:instancePath+"/result/spaces/" + i0+"/space",schemaPath:"#/$defs/SyncEntry/properties/space/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid4 = _errs27 === errors;
}
else {
var valid4 = true;
}
if(valid4){
if(data6.writable !== undefined){
const _errs29 = errors;
if(typeof data6.writable !== "boolean"){
validate201.errors = [{instancePath:instancePath+"/result/spaces/" + i0+"/writable",schemaPath:"#/$defs/SyncEntry/properties/writable/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid4 = _errs29 === errors;
}
else {
var valid4 = true;
}
}
}
}
}
}
}
}
else {
validate201.errors = [{instancePath:instancePath+"/result/spaces/" + i0,schemaPath:"#/$defs/SyncEntry/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
var valid2 = _errs15 === errors;
if(!valid2){
break;
}
}
}
else {
validate201.errors = [{instancePath:instancePath+"/result/spaces",schemaPath:"#/properties/result/properties/spaces/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs13 === errors;
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
else {
validate201.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/type",keyword:"type",params:{type: "object"},message:"must be object"}];
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
let data13 = data.status;
const _errs31 = errors;
if(typeof data13 !== "string"){
validate201.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data13 === "ok")){
validate201.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/enum",keyword:"enum",params:{allowedValues: schema44.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs31 === errors;
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
validate201.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate201.errors = vErrors;
return errors === 0;
}
validate201.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate200(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate200.evaluated;
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
if(!(validate201(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate201.errors : vErrors.concat(validate201.errors);
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
validate200.errors = vErrors;
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
validate200.errors = vErrors;
evaluated0.props = props0;
return errors === 0;
}
validate200.evaluated = {"dynamicProps":true,"dynamicItems":false};

export const CommandResponse_SyncControl = validate204;
const schema197 = {"oneOf":[{"$ref":"#/$defs/CommandSuccess_SyncControl"},{"$ref":"#/$defs/CommandFailure"}],"description":"Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.\nA success confirms the original completion, not the current object state.\nConstruct success only AFTER the storage transaction commits. Cancellation,\na lost response or uncertain commit must never be converted to success."};
const schema198 = {"type":"object","required":["status","result","replayed"],"properties":{"replayed":{"type":"boolean"},"result":{"type":"object","required":["space","pairing","selected","paused","selection_version","pause_version"],"properties":{"pairing":{"type":"string"},"pause_version":{"$ref":"#/$defs/StateVersion"},"paused":{"type":"boolean"},"selected":{"type":"boolean"},"selection_version":{"$ref":"#/$defs/StateVersion"},"space":{"type":"string"}},"additionalProperties":false},"status":{"$ref":"#/$defs/SuccessStatus"}},"additionalProperties":false};

function validate205(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate205.evaluated;
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
validate205.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((key0 === "replayed") || (key0 === "result")) || (key0 === "status"))){
validate205.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.replayed !== undefined){
const _errs2 = errors;
if(typeof data.replayed !== "boolean"){
validate205.errors = [{instancePath:instancePath+"/replayed",schemaPath:"#/properties/replayed/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
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
if(((((((data1.space === undefined) && (missing1 = "space")) || ((data1.pairing === undefined) && (missing1 = "pairing"))) || ((data1.selected === undefined) && (missing1 = "selected"))) || ((data1.paused === undefined) && (missing1 = "paused"))) || ((data1.selection_version === undefined) && (missing1 = "selection_version"))) || ((data1.pause_version === undefined) && (missing1 = "pause_version"))){
validate205.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/required",keyword:"required",params:{missingProperty: missing1},message:"must have required property '"+missing1+"'"}];
return false;
}
else {
const _errs6 = errors;
for(const key1 in data1){
if(!((((((key1 === "pairing") || (key1 === "pause_version")) || (key1 === "paused")) || (key1 === "selected")) || (key1 === "selection_version")) || (key1 === "space"))){
validate205.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key1},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs6 === errors){
if(data1.pairing !== undefined){
const _errs7 = errors;
if(typeof data1.pairing !== "string"){
validate205.errors = [{instancePath:instancePath+"/result/pairing",schemaPath:"#/properties/result/properties/pairing/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid1 = _errs7 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.pause_version !== undefined){
let data3 = data1.pause_version;
const _errs9 = errors;
const _errs10 = errors;
if(errors === _errs10){
if(Array.isArray(data3)){
if(data3.length > 32){
validate205.errors = [{instancePath:instancePath+"/result/pause_version",schemaPath:"#/$defs/StateVersion/maxItems",keyword:"maxItems",params:{limit: 32},message:"must NOT have more than 32 items"}];
return false;
}
else {
if(data3.length < 32){
validate205.errors = [{instancePath:instancePath+"/result/pause_version",schemaPath:"#/$defs/StateVersion/minItems",keyword:"minItems",params:{limit: 32},message:"must NOT have fewer than 32 items"}];
return false;
}
else {
var valid3 = true;
const len0 = data3.length;
for(let i0=0; i0<len0; i0++){
let data4 = data3[i0];
const _errs12 = errors;
if(!((typeof data4 == "number") && (!(data4 % 1) && !isNaN(data4)))){
validate205.errors = [{instancePath:instancePath+"/result/pause_version/" + i0,schemaPath:"#/$defs/StateVersion/items/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs12){
if(typeof data4 == "number"){
if(data4 > 255 || isNaN(data4)){
validate205.errors = [{instancePath:instancePath+"/result/pause_version/" + i0,schemaPath:"#/$defs/StateVersion/items/maximum",keyword:"maximum",params:{comparison: "<=", limit: 255},message:"must be <= 255"}];
return false;
}
else {
if(data4 < 0 || isNaN(data4)){
validate205.errors = [{instancePath:instancePath+"/result/pause_version/" + i0,schemaPath:"#/$defs/StateVersion/items/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
}
var valid3 = _errs12 === errors;
if(!valid3){
break;
}
}
}
}
}
else {
validate205.errors = [{instancePath:instancePath+"/result/pause_version",schemaPath:"#/$defs/StateVersion/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs9 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.paused !== undefined){
const _errs14 = errors;
if(typeof data1.paused !== "boolean"){
validate205.errors = [{instancePath:instancePath+"/result/paused",schemaPath:"#/properties/result/properties/paused/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid1 = _errs14 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.selected !== undefined){
const _errs16 = errors;
if(typeof data1.selected !== "boolean"){
validate205.errors = [{instancePath:instancePath+"/result/selected",schemaPath:"#/properties/result/properties/selected/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid1 = _errs16 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.selection_version !== undefined){
let data7 = data1.selection_version;
const _errs18 = errors;
const _errs19 = errors;
if(errors === _errs19){
if(Array.isArray(data7)){
if(data7.length > 32){
validate205.errors = [{instancePath:instancePath+"/result/selection_version",schemaPath:"#/$defs/StateVersion/maxItems",keyword:"maxItems",params:{limit: 32},message:"must NOT have more than 32 items"}];
return false;
}
else {
if(data7.length < 32){
validate205.errors = [{instancePath:instancePath+"/result/selection_version",schemaPath:"#/$defs/StateVersion/minItems",keyword:"minItems",params:{limit: 32},message:"must NOT have fewer than 32 items"}];
return false;
}
else {
var valid5 = true;
const len1 = data7.length;
for(let i1=0; i1<len1; i1++){
let data8 = data7[i1];
const _errs21 = errors;
if(!((typeof data8 == "number") && (!(data8 % 1) && !isNaN(data8)))){
validate205.errors = [{instancePath:instancePath+"/result/selection_version/" + i1,schemaPath:"#/$defs/StateVersion/items/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs21){
if(typeof data8 == "number"){
if(data8 > 255 || isNaN(data8)){
validate205.errors = [{instancePath:instancePath+"/result/selection_version/" + i1,schemaPath:"#/$defs/StateVersion/items/maximum",keyword:"maximum",params:{comparison: "<=", limit: 255},message:"must be <= 255"}];
return false;
}
else {
if(data8 < 0 || isNaN(data8)){
validate205.errors = [{instancePath:instancePath+"/result/selection_version/" + i1,schemaPath:"#/$defs/StateVersion/items/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
}
var valid5 = _errs21 === errors;
if(!valid5){
break;
}
}
}
}
}
else {
validate205.errors = [{instancePath:instancePath+"/result/selection_version",schemaPath:"#/$defs/StateVersion/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs18 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.space !== undefined){
const _errs23 = errors;
if(typeof data1.space !== "string"){
validate205.errors = [{instancePath:instancePath+"/result/space",schemaPath:"#/properties/result/properties/space/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid1 = _errs23 === errors;
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
validate205.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/type",keyword:"type",params:{type: "object"},message:"must be object"}];
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
let data10 = data.status;
const _errs25 = errors;
if(typeof data10 !== "string"){
validate205.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data10 === "ok")){
validate205.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/enum",keyword:"enum",params:{allowedValues: schema44.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs25 === errors;
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
validate205.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate205.errors = vErrors;
return errors === 0;
}
validate205.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate204(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate204.evaluated;
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
if(!(validate205(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate205.errors : vErrors.concat(validate205.errors);
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
validate204.errors = vErrors;
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
validate204.errors = vErrors;
evaluated0.props = props0;
return errors === 0;
}
validate204.evaluated = {"dynamicProps":true,"dynamicItems":false};

export const CommandResponse_SyncPage = validate208;
const schema202 = {"oneOf":[{"$ref":"#/$defs/CommandSuccess_SyncPage"},{"$ref":"#/$defs/CommandFailure"}],"description":"Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.\nA success confirms the original completion, not the current object state.\nConstruct success only AFTER the storage transaction commits. Cancellation,\na lost response or uncertain commit must never be converted to success."};
const schema203 = {"type":"object","required":["status","result","replayed"],"properties":{"replayed":{"type":"boolean"},"result":{"type":"object","required":["protocol","space","base","heads","changes","complete"],"properties":{"base":{"type":"array","items":{"type":"array","items":{"type":"integer","maximum":255,"minimum":0},"maxItems":32,"minItems":32},"maxItems":1024},"changes":{"type":"array","items":{"type":"string"}},"complete":{"type":"boolean"},"heads":{"type":"array","items":{"type":"array","items":{"type":"integer","maximum":255,"minimum":0},"maxItems":32,"minItems":32},"maxItems":1024},"protocol":{"type":"integer","format":"int32","minimum":0},"space":{"type":"string"}},"additionalProperties":false},"status":{"$ref":"#/$defs/SuccessStatus"}},"additionalProperties":false};

function validate209(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate209.evaluated;
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
validate209.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((key0 === "replayed") || (key0 === "result")) || (key0 === "status"))){
validate209.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.replayed !== undefined){
const _errs2 = errors;
if(typeof data.replayed !== "boolean"){
validate209.errors = [{instancePath:instancePath+"/replayed",schemaPath:"#/properties/replayed/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
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
if(((((((data1.protocol === undefined) && (missing1 = "protocol")) || ((data1.space === undefined) && (missing1 = "space"))) || ((data1.base === undefined) && (missing1 = "base"))) || ((data1.heads === undefined) && (missing1 = "heads"))) || ((data1.changes === undefined) && (missing1 = "changes"))) || ((data1.complete === undefined) && (missing1 = "complete"))){
validate209.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/required",keyword:"required",params:{missingProperty: missing1},message:"must have required property '"+missing1+"'"}];
return false;
}
else {
const _errs6 = errors;
for(const key1 in data1){
if(!((((((key1 === "base") || (key1 === "changes")) || (key1 === "complete")) || (key1 === "heads")) || (key1 === "protocol")) || (key1 === "space"))){
validate209.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key1},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs6 === errors){
if(data1.base !== undefined){
let data2 = data1.base;
const _errs7 = errors;
if(errors === _errs7){
if(Array.isArray(data2)){
if(data2.length > 1024){
validate209.errors = [{instancePath:instancePath+"/result/base",schemaPath:"#/properties/result/properties/base/maxItems",keyword:"maxItems",params:{limit: 1024},message:"must NOT have more than 1024 items"}];
return false;
}
else {
var valid2 = true;
const len0 = data2.length;
for(let i0=0; i0<len0; i0++){
let data3 = data2[i0];
const _errs9 = errors;
if(errors === _errs9){
if(Array.isArray(data3)){
if(data3.length > 32){
validate209.errors = [{instancePath:instancePath+"/result/base/" + i0,schemaPath:"#/properties/result/properties/base/items/maxItems",keyword:"maxItems",params:{limit: 32},message:"must NOT have more than 32 items"}];
return false;
}
else {
if(data3.length < 32){
validate209.errors = [{instancePath:instancePath+"/result/base/" + i0,schemaPath:"#/properties/result/properties/base/items/minItems",keyword:"minItems",params:{limit: 32},message:"must NOT have fewer than 32 items"}];
return false;
}
else {
var valid3 = true;
const len1 = data3.length;
for(let i1=0; i1<len1; i1++){
let data4 = data3[i1];
const _errs11 = errors;
if(!((typeof data4 == "number") && (!(data4 % 1) && !isNaN(data4)))){
validate209.errors = [{instancePath:instancePath+"/result/base/" + i0+"/" + i1,schemaPath:"#/properties/result/properties/base/items/items/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs11){
if(typeof data4 == "number"){
if(data4 > 255 || isNaN(data4)){
validate209.errors = [{instancePath:instancePath+"/result/base/" + i0+"/" + i1,schemaPath:"#/properties/result/properties/base/items/items/maximum",keyword:"maximum",params:{comparison: "<=", limit: 255},message:"must be <= 255"}];
return false;
}
else {
if(data4 < 0 || isNaN(data4)){
validate209.errors = [{instancePath:instancePath+"/result/base/" + i0+"/" + i1,schemaPath:"#/properties/result/properties/base/items/items/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
}
var valid3 = _errs11 === errors;
if(!valid3){
break;
}
}
}
}
}
else {
validate209.errors = [{instancePath:instancePath+"/result/base/" + i0,schemaPath:"#/properties/result/properties/base/items/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid2 = _errs9 === errors;
if(!valid2){
break;
}
}
}
}
else {
validate209.errors = [{instancePath:instancePath+"/result/base",schemaPath:"#/properties/result/properties/base/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs7 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.changes !== undefined){
let data5 = data1.changes;
const _errs13 = errors;
if(errors === _errs13){
if(Array.isArray(data5)){
var valid4 = true;
const len2 = data5.length;
for(let i2=0; i2<len2; i2++){
const _errs15 = errors;
if(typeof data5[i2] !== "string"){
validate209.errors = [{instancePath:instancePath+"/result/changes/" + i2,schemaPath:"#/properties/result/properties/changes/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid4 = _errs15 === errors;
if(!valid4){
break;
}
}
}
else {
validate209.errors = [{instancePath:instancePath+"/result/changes",schemaPath:"#/properties/result/properties/changes/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs13 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.complete !== undefined){
const _errs17 = errors;
if(typeof data1.complete !== "boolean"){
validate209.errors = [{instancePath:instancePath+"/result/complete",schemaPath:"#/properties/result/properties/complete/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid1 = _errs17 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.heads !== undefined){
let data8 = data1.heads;
const _errs19 = errors;
if(errors === _errs19){
if(Array.isArray(data8)){
if(data8.length > 1024){
validate209.errors = [{instancePath:instancePath+"/result/heads",schemaPath:"#/properties/result/properties/heads/maxItems",keyword:"maxItems",params:{limit: 1024},message:"must NOT have more than 1024 items"}];
return false;
}
else {
var valid5 = true;
const len3 = data8.length;
for(let i3=0; i3<len3; i3++){
let data9 = data8[i3];
const _errs21 = errors;
if(errors === _errs21){
if(Array.isArray(data9)){
if(data9.length > 32){
validate209.errors = [{instancePath:instancePath+"/result/heads/" + i3,schemaPath:"#/properties/result/properties/heads/items/maxItems",keyword:"maxItems",params:{limit: 32},message:"must NOT have more than 32 items"}];
return false;
}
else {
if(data9.length < 32){
validate209.errors = [{instancePath:instancePath+"/result/heads/" + i3,schemaPath:"#/properties/result/properties/heads/items/minItems",keyword:"minItems",params:{limit: 32},message:"must NOT have fewer than 32 items"}];
return false;
}
else {
var valid6 = true;
const len4 = data9.length;
for(let i4=0; i4<len4; i4++){
let data10 = data9[i4];
const _errs23 = errors;
if(!((typeof data10 == "number") && (!(data10 % 1) && !isNaN(data10)))){
validate209.errors = [{instancePath:instancePath+"/result/heads/" + i3+"/" + i4,schemaPath:"#/properties/result/properties/heads/items/items/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs23){
if(typeof data10 == "number"){
if(data10 > 255 || isNaN(data10)){
validate209.errors = [{instancePath:instancePath+"/result/heads/" + i3+"/" + i4,schemaPath:"#/properties/result/properties/heads/items/items/maximum",keyword:"maximum",params:{comparison: "<=", limit: 255},message:"must be <= 255"}];
return false;
}
else {
if(data10 < 0 || isNaN(data10)){
validate209.errors = [{instancePath:instancePath+"/result/heads/" + i3+"/" + i4,schemaPath:"#/properties/result/properties/heads/items/items/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
}
var valid6 = _errs23 === errors;
if(!valid6){
break;
}
}
}
}
}
else {
validate209.errors = [{instancePath:instancePath+"/result/heads/" + i3,schemaPath:"#/properties/result/properties/heads/items/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid5 = _errs21 === errors;
if(!valid5){
break;
}
}
}
}
else {
validate209.errors = [{instancePath:instancePath+"/result/heads",schemaPath:"#/properties/result/properties/heads/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs19 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.protocol !== undefined){
let data11 = data1.protocol;
const _errs25 = errors;
if(!((typeof data11 == "number") && (!(data11 % 1) && !isNaN(data11)))){
validate209.errors = [{instancePath:instancePath+"/result/protocol",schemaPath:"#/properties/result/properties/protocol/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs25){
if(typeof data11 == "number"){
if(data11 < 0 || isNaN(data11)){
validate209.errors = [{instancePath:instancePath+"/result/protocol",schemaPath:"#/properties/result/properties/protocol/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
var valid1 = _errs25 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.space !== undefined){
const _errs27 = errors;
if(typeof data1.space !== "string"){
validate209.errors = [{instancePath:instancePath+"/result/space",schemaPath:"#/properties/result/properties/space/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid1 = _errs27 === errors;
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
validate209.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/type",keyword:"type",params:{type: "object"},message:"must be object"}];
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
let data13 = data.status;
const _errs29 = errors;
if(typeof data13 !== "string"){
validate209.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data13 === "ok")){
validate209.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/enum",keyword:"enum",params:{allowedValues: schema44.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs29 === errors;
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
validate209.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate209.errors = vErrors;
return errors === 0;
}
validate209.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate208(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate208.evaluated;
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
if(!(validate209(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate209.errors : vErrors.concat(validate209.errors);
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
validate208.errors = vErrors;
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
validate208.errors = vErrors;
evaluated0.props = props0;
return errors === 0;
}
validate208.evaluated = {"dynamicProps":true,"dynamicItems":false};

export const CommandResponse_TagSnapshot = validate212;
const schema205 = {"oneOf":[{"$ref":"#/$defs/CommandSuccess_TagSnapshot"},{"$ref":"#/$defs/CommandFailure"}],"description":"Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.\nA success confirms the original completion, not the current object state.\nConstruct success only AFTER the storage transaction commits. Cancellation,\na lost response or uncertain commit must never be converted to success."};
const schema206 = {"type":"object","required":["status","result","replayed"],"properties":{"replayed":{"type":"boolean"},"result":{"type":"object","required":["space","tag","names","state","version"],"properties":{"deletion_conflict":{"type":"boolean","description":"True when deletion competes with independent assignment or a different deletion decision. Resolve with tag.resolve_deletion and a fresh version; ordinary writes remain blocked."},"name_group":{"type":"array","items":{"type":"string"}},"names":{"type":"array","items":{"type":"string"}},"space":{"type":"string"},"state":{"$ref":"#/$defs/TagState"},"tag":{"type":"string"},"version":{"$ref":"#/$defs/StateVersion","description":"Includes causal assignment history, so deletion cannot expand silently."}},"additionalProperties":false},"status":{"$ref":"#/$defs/SuccessStatus"}},"additionalProperties":false};

function validate213(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate213.evaluated;
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
validate213.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((key0 === "replayed") || (key0 === "result")) || (key0 === "status"))){
validate213.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.replayed !== undefined){
const _errs2 = errors;
if(typeof data.replayed !== "boolean"){
validate213.errors = [{instancePath:instancePath+"/replayed",schemaPath:"#/properties/replayed/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
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
if((((((data1.space === undefined) && (missing1 = "space")) || ((data1.tag === undefined) && (missing1 = "tag"))) || ((data1.names === undefined) && (missing1 = "names"))) || ((data1.state === undefined) && (missing1 = "state"))) || ((data1.version === undefined) && (missing1 = "version"))){
validate213.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/required",keyword:"required",params:{missingProperty: missing1},message:"must have required property '"+missing1+"'"}];
return false;
}
else {
const _errs6 = errors;
for(const key1 in data1){
if(!(((((((key1 === "deletion_conflict") || (key1 === "name_group")) || (key1 === "names")) || (key1 === "space")) || (key1 === "state")) || (key1 === "tag")) || (key1 === "version"))){
validate213.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key1},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs6 === errors){
if(data1.deletion_conflict !== undefined){
const _errs7 = errors;
if(typeof data1.deletion_conflict !== "boolean"){
validate213.errors = [{instancePath:instancePath+"/result/deletion_conflict",schemaPath:"#/properties/result/properties/deletion_conflict/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid1 = _errs7 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.name_group !== undefined){
let data3 = data1.name_group;
const _errs9 = errors;
if(errors === _errs9){
if(Array.isArray(data3)){
var valid2 = true;
const len0 = data3.length;
for(let i0=0; i0<len0; i0++){
const _errs11 = errors;
if(typeof data3[i0] !== "string"){
validate213.errors = [{instancePath:instancePath+"/result/name_group/" + i0,schemaPath:"#/properties/result/properties/name_group/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid2 = _errs11 === errors;
if(!valid2){
break;
}
}
}
else {
validate213.errors = [{instancePath:instancePath+"/result/name_group",schemaPath:"#/properties/result/properties/name_group/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs9 === errors;
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
validate213.errors = [{instancePath:instancePath+"/result/names/" + i1,schemaPath:"#/properties/result/properties/names/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid3 = _errs15 === errors;
if(!valid3){
break;
}
}
}
else {
validate213.errors = [{instancePath:instancePath+"/result/names",schemaPath:"#/properties/result/properties/names/type",keyword:"type",params:{type: "array"},message:"must be array"}];
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
validate213.errors = [{instancePath:instancePath+"/result/space",schemaPath:"#/properties/result/properties/space/type",keyword:"type",params:{type: "string"},message:"must be string"}];
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
validate213.errors = [{instancePath:instancePath+"/result/state",schemaPath:"#/$defs/TagState/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(((data8 === "active") || (data8 === "deleted")) || (data8 === "conflicted"))){
validate213.errors = [{instancePath:instancePath+"/result/state",schemaPath:"#/$defs/TagState/enum",keyword:"enum",params:{allowedValues: schema110.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid1 = _errs19 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.tag !== undefined){
const _errs22 = errors;
if(typeof data1.tag !== "string"){
validate213.errors = [{instancePath:instancePath+"/result/tag",schemaPath:"#/properties/result/properties/tag/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid1 = _errs22 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.version !== undefined){
let data10 = data1.version;
const _errs24 = errors;
const _errs25 = errors;
if(errors === _errs25){
if(Array.isArray(data10)){
if(data10.length > 32){
validate213.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/maxItems",keyword:"maxItems",params:{limit: 32},message:"must NOT have more than 32 items"}];
return false;
}
else {
if(data10.length < 32){
validate213.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/minItems",keyword:"minItems",params:{limit: 32},message:"must NOT have fewer than 32 items"}];
return false;
}
else {
var valid6 = true;
const len2 = data10.length;
for(let i2=0; i2<len2; i2++){
let data11 = data10[i2];
const _errs27 = errors;
if(!((typeof data11 == "number") && (!(data11 % 1) && !isNaN(data11)))){
validate213.errors = [{instancePath:instancePath+"/result/version/" + i2,schemaPath:"#/$defs/StateVersion/items/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs27){
if(typeof data11 == "number"){
if(data11 > 255 || isNaN(data11)){
validate213.errors = [{instancePath:instancePath+"/result/version/" + i2,schemaPath:"#/$defs/StateVersion/items/maximum",keyword:"maximum",params:{comparison: "<=", limit: 255},message:"must be <= 255"}];
return false;
}
else {
if(data11 < 0 || isNaN(data11)){
validate213.errors = [{instancePath:instancePath+"/result/version/" + i2,schemaPath:"#/$defs/StateVersion/items/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
}
var valid6 = _errs27 === errors;
if(!valid6){
break;
}
}
}
}
}
else {
validate213.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs24 === errors;
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
else {
validate213.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/type",keyword:"type",params:{type: "object"},message:"must be object"}];
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
let data12 = data.status;
const _errs29 = errors;
if(typeof data12 !== "string"){
validate213.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data12 === "ok")){
validate213.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/enum",keyword:"enum",params:{allowedValues: schema44.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs29 === errors;
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
validate213.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate213.errors = vErrors;
return errors === 0;
}
validate213.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate212(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate212.evaluated;
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
if(!(validate213(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate213.errors : vErrors.concat(validate213.errors);
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
validate212.errors = vErrors;
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
validate212.errors = vErrors;
evaluated0.props = props0;
return errors === 0;
}
validate212.evaluated = {"dynamicProps":true,"dynamicItems":false};

export const CommandResponse_TagList = validate216;
const schema210 = {"oneOf":[{"$ref":"#/$defs/CommandSuccess_TagList"},{"$ref":"#/$defs/CommandFailure"}],"description":"Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.\nA success confirms the original completion, not the current object state.\nConstruct success only AFTER the storage transaction commits. Cancellation,\na lost response or uncertain commit must never be converted to success."};
const schema211 = {"type":"object","required":["status","result","replayed"],"properties":{"replayed":{"type":"boolean"},"result":{"type":"object","required":["tags"],"properties":{"tags":{"type":"array","items":{"$ref":"#/$defs/TagSnapshot"}}},"additionalProperties":false},"status":{"$ref":"#/$defs/SuccessStatus"}},"additionalProperties":false};

function validate217(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate217.evaluated;
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
validate217.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((key0 === "replayed") || (key0 === "result")) || (key0 === "status"))){
validate217.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.replayed !== undefined){
const _errs2 = errors;
if(typeof data.replayed !== "boolean"){
validate217.errors = [{instancePath:instancePath+"/replayed",schemaPath:"#/properties/replayed/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
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
if((data1.tags === undefined) && (missing1 = "tags")){
validate217.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/required",keyword:"required",params:{missingProperty: missing1},message:"must have required property '"+missing1+"'"}];
return false;
}
else {
const _errs6 = errors;
for(const key1 in data1){
if(!(key1 === "tags")){
validate217.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key1},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs6 === errors){
if(data1.tags !== undefined){
let data2 = data1.tags;
const _errs7 = errors;
if(errors === _errs7){
if(Array.isArray(data2)){
var valid2 = true;
const len0 = data2.length;
for(let i0=0; i0<len0; i0++){
const _errs9 = errors;
if(!(validate112(data2[i0], {instancePath:instancePath+"/result/tags/" + i0,parentData:data2,parentDataProperty:i0,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate112.errors : vErrors.concat(validate112.errors);
errors = vErrors.length;
}
var valid2 = _errs9 === errors;
if(!valid2){
break;
}
}
}
else {
validate217.errors = [{instancePath:instancePath+"/result/tags",schemaPath:"#/properties/result/properties/tags/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
}
}
}
}
else {
validate217.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/type",keyword:"type",params:{type: "object"},message:"must be object"}];
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
validate217.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data4 === "ok")){
validate217.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/enum",keyword:"enum",params:{allowedValues: schema44.enum},message:"must be equal to one of the allowed values"}];
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
validate217.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate217.errors = vErrors;
return errors === 0;
}
validate217.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate216(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate216.evaluated;
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
if(!(validate217(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate217.errors : vErrors.concat(validate217.errors);
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
validate216.errors = vErrors;
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
validate216.errors = vErrors;
evaluated0.props = props0;
return errors === 0;
}
validate216.evaluated = {"dynamicProps":true,"dynamicItems":false};

export const CommandResponse_TrashPreview = validate221;
const schema213 = {"oneOf":[{"$ref":"#/$defs/CommandSuccess_TrashPreview"},{"$ref":"#/$defs/CommandFailure"}],"description":"Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.\nA success confirms the original completion, not the current object state.\nConstruct success only AFTER the storage transaction commits. Cancellation,\na lost response or uncertain commit must never be converted to success."};
const schema214 = {"type":"object","required":["status","result","replayed"],"properties":{"replayed":{"type":"boolean"},"result":{"type":"object","required":["space","selection","includes_space","columns","cards","conflicts","version"],"properties":{"cards":{"type":"array","items":{"$ref":"#/$defs/CardSnapshot"}},"columns":{"type":"array","items":{"$ref":"#/$defs/ColumnSnapshot"}},"conflicts":{"type":"array","items":{"type":"string"},"description":"Stable object identities to resolve before preparing a new action."},"includes_space":{"type":"boolean"},"selection":{"$ref":"#/$defs/TrashSelection"},"space":{"type":"string"},"version":{"$ref":"#/$defs/StateVersion"}},"additionalProperties":false},"status":{"$ref":"#/$defs/SuccessStatus"}},"additionalProperties":false};
const schema215 = {"oneOf":[{"type":"object","required":["kind"],"properties":{"kind":{"type":"string","enum":["all"]}}},{"type":"object","required":["kind"],"properties":{"kind":{"type":"string","enum":["space"]}}},{"type":"object","required":["columns","cards","kind"],"properties":{"cards":{"type":"array","items":{"type":"string"}},"columns":{"type":"array","items":{"type":"string"}},"kind":{"type":"string","enum":["objects"]}}}]};

function validate222(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate222.evaluated;
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
validate222.errors = [{instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: missing0},message:"must have required property '"+missing0+"'"}];
return false;
}
else {
const _errs1 = errors;
for(const key0 in data){
if(!(((key0 === "replayed") || (key0 === "result")) || (key0 === "status"))){
validate222.errors = [{instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"}];
return false;
break;
}
}
if(_errs1 === errors){
if(data.replayed !== undefined){
const _errs2 = errors;
if(typeof data.replayed !== "boolean"){
validate222.errors = [{instancePath:instancePath+"/replayed",schemaPath:"#/properties/replayed/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
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
if((((((((data1.space === undefined) && (missing1 = "space")) || ((data1.selection === undefined) && (missing1 = "selection"))) || ((data1.includes_space === undefined) && (missing1 = "includes_space"))) || ((data1.columns === undefined) && (missing1 = "columns"))) || ((data1.cards === undefined) && (missing1 = "cards"))) || ((data1.conflicts === undefined) && (missing1 = "conflicts"))) || ((data1.version === undefined) && (missing1 = "version"))){
validate222.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/required",keyword:"required",params:{missingProperty: missing1},message:"must have required property '"+missing1+"'"}];
return false;
}
else {
const _errs6 = errors;
for(const key1 in data1){
if(!(((((((key1 === "cards") || (key1 === "columns")) || (key1 === "conflicts")) || (key1 === "includes_space")) || (key1 === "selection")) || (key1 === "space")) || (key1 === "version"))){
validate222.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key1},message:"must NOT have additional properties"}];
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
if(!(validate80(data2[i0], {instancePath:instancePath+"/result/cards/" + i0,parentData:data2,parentDataProperty:i0,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate80.errors : vErrors.concat(validate80.errors);
errors = vErrors.length;
}
var valid2 = _errs9 === errors;
if(!valid2){
break;
}
}
}
else {
validate222.errors = [{instancePath:instancePath+"/result/cards",schemaPath:"#/properties/result/properties/cards/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs7 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.columns !== undefined){
let data4 = data1.columns;
const _errs10 = errors;
if(errors === _errs10){
if(Array.isArray(data4)){
var valid3 = true;
const len1 = data4.length;
for(let i1=0; i1<len1; i1++){
const _errs12 = errors;
if(!(validate108(data4[i1], {instancePath:instancePath+"/result/columns/" + i1,parentData:data4,parentDataProperty:i1,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate108.errors : vErrors.concat(validate108.errors);
errors = vErrors.length;
}
var valid3 = _errs12 === errors;
if(!valid3){
break;
}
}
}
else {
validate222.errors = [{instancePath:instancePath+"/result/columns",schemaPath:"#/properties/result/properties/columns/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs10 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.conflicts !== undefined){
let data6 = data1.conflicts;
const _errs13 = errors;
if(errors === _errs13){
if(Array.isArray(data6)){
var valid4 = true;
const len2 = data6.length;
for(let i2=0; i2<len2; i2++){
const _errs15 = errors;
if(typeof data6[i2] !== "string"){
validate222.errors = [{instancePath:instancePath+"/result/conflicts/" + i2,schemaPath:"#/properties/result/properties/conflicts/items/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid4 = _errs15 === errors;
if(!valid4){
break;
}
}
}
else {
validate222.errors = [{instancePath:instancePath+"/result/conflicts",schemaPath:"#/properties/result/properties/conflicts/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs13 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.includes_space !== undefined){
const _errs17 = errors;
if(typeof data1.includes_space !== "boolean"){
validate222.errors = [{instancePath:instancePath+"/result/includes_space",schemaPath:"#/properties/result/properties/includes_space/type",keyword:"type",params:{type: "boolean"},message:"must be boolean"}];
return false;
}
var valid1 = _errs17 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.selection !== undefined){
let data9 = data1.selection;
const _errs19 = errors;
const _errs21 = errors;
let valid6 = false;
let passing0 = null;
const _errs22 = errors;
if(errors === _errs22){
if(data9 && typeof data9 == "object" && !Array.isArray(data9)){
let missing2;
if((data9.kind === undefined) && (missing2 = "kind")){
const err0 = {instancePath:instancePath+"/result/selection",schemaPath:"#/$defs/TrashSelection/oneOf/0/required",keyword:"required",params:{missingProperty: missing2},message:"must have required property '"+missing2+"'"};
if(vErrors === null){
vErrors = [err0];
}
else {
vErrors.push(err0);
}
errors++;
}
else {
if(data9.kind !== undefined){
let data10 = data9.kind;
if(typeof data10 !== "string"){
const err1 = {instancePath:instancePath+"/result/selection/kind",schemaPath:"#/$defs/TrashSelection/oneOf/0/properties/kind/type",keyword:"type",params:{type: "string"},message:"must be string"};
if(vErrors === null){
vErrors = [err1];
}
else {
vErrors.push(err1);
}
errors++;
}
if(!(data10 === "all")){
const err2 = {instancePath:instancePath+"/result/selection/kind",schemaPath:"#/$defs/TrashSelection/oneOf/0/properties/kind/enum",keyword:"enum",params:{allowedValues: schema215.oneOf[0].properties.kind.enum},message:"must be equal to one of the allowed values"};
if(vErrors === null){
vErrors = [err2];
}
else {
vErrors.push(err2);
}
errors++;
}
}
}
}
else {
const err3 = {instancePath:instancePath+"/result/selection",schemaPath:"#/$defs/TrashSelection/oneOf/0/type",keyword:"type",params:{type: "object"},message:"must be object"};
if(vErrors === null){
vErrors = [err3];
}
else {
vErrors.push(err3);
}
errors++;
}
}
var _valid0 = _errs22 === errors;
if(_valid0){
valid6 = true;
passing0 = 0;
var props0 = {};
props0.kind = true;
}
const _errs26 = errors;
if(errors === _errs26){
if(data9 && typeof data9 == "object" && !Array.isArray(data9)){
let missing3;
if((data9.kind === undefined) && (missing3 = "kind")){
const err4 = {instancePath:instancePath+"/result/selection",schemaPath:"#/$defs/TrashSelection/oneOf/1/required",keyword:"required",params:{missingProperty: missing3},message:"must have required property '"+missing3+"'"};
if(vErrors === null){
vErrors = [err4];
}
else {
vErrors.push(err4);
}
errors++;
}
else {
if(data9.kind !== undefined){
let data11 = data9.kind;
if(typeof data11 !== "string"){
const err5 = {instancePath:instancePath+"/result/selection/kind",schemaPath:"#/$defs/TrashSelection/oneOf/1/properties/kind/type",keyword:"type",params:{type: "string"},message:"must be string"};
if(vErrors === null){
vErrors = [err5];
}
else {
vErrors.push(err5);
}
errors++;
}
if(!(data11 === "space")){
const err6 = {instancePath:instancePath+"/result/selection/kind",schemaPath:"#/$defs/TrashSelection/oneOf/1/properties/kind/enum",keyword:"enum",params:{allowedValues: schema215.oneOf[1].properties.kind.enum},message:"must be equal to one of the allowed values"};
if(vErrors === null){
vErrors = [err6];
}
else {
vErrors.push(err6);
}
errors++;
}
}
}
}
else {
const err7 = {instancePath:instancePath+"/result/selection",schemaPath:"#/$defs/TrashSelection/oneOf/1/type",keyword:"type",params:{type: "object"},message:"must be object"};
if(vErrors === null){
vErrors = [err7];
}
else {
vErrors.push(err7);
}
errors++;
}
}
var _valid0 = _errs26 === errors;
if(_valid0 && valid6){
valid6 = false;
passing0 = [passing0, 1];
}
else {
if(_valid0){
valid6 = true;
passing0 = 1;
if(props0 !== true){
props0 = props0 || {};
props0.kind = true;
}
}
const _errs30 = errors;
if(errors === _errs30){
if(data9 && typeof data9 == "object" && !Array.isArray(data9)){
let missing4;
if((((data9.columns === undefined) && (missing4 = "columns")) || ((data9.cards === undefined) && (missing4 = "cards"))) || ((data9.kind === undefined) && (missing4 = "kind"))){
const err8 = {instancePath:instancePath+"/result/selection",schemaPath:"#/$defs/TrashSelection/oneOf/2/required",keyword:"required",params:{missingProperty: missing4},message:"must have required property '"+missing4+"'"};
if(vErrors === null){
vErrors = [err8];
}
else {
vErrors.push(err8);
}
errors++;
}
else {
if(data9.cards !== undefined){
let data12 = data9.cards;
const _errs32 = errors;
if(errors === _errs32){
if(Array.isArray(data12)){
var valid10 = true;
const len3 = data12.length;
for(let i3=0; i3<len3; i3++){
const _errs34 = errors;
if(typeof data12[i3] !== "string"){
const err9 = {instancePath:instancePath+"/result/selection/cards/" + i3,schemaPath:"#/$defs/TrashSelection/oneOf/2/properties/cards/items/type",keyword:"type",params:{type: "string"},message:"must be string"};
if(vErrors === null){
vErrors = [err9];
}
else {
vErrors.push(err9);
}
errors++;
}
var valid10 = _errs34 === errors;
if(!valid10){
break;
}
}
}
else {
const err10 = {instancePath:instancePath+"/result/selection/cards",schemaPath:"#/$defs/TrashSelection/oneOf/2/properties/cards/type",keyword:"type",params:{type: "array"},message:"must be array"};
if(vErrors === null){
vErrors = [err10];
}
else {
vErrors.push(err10);
}
errors++;
}
}
var valid9 = _errs32 === errors;
}
else {
var valid9 = true;
}
if(valid9){
if(data9.columns !== undefined){
let data14 = data9.columns;
const _errs36 = errors;
if(errors === _errs36){
if(Array.isArray(data14)){
var valid11 = true;
const len4 = data14.length;
for(let i4=0; i4<len4; i4++){
const _errs38 = errors;
if(typeof data14[i4] !== "string"){
const err11 = {instancePath:instancePath+"/result/selection/columns/" + i4,schemaPath:"#/$defs/TrashSelection/oneOf/2/properties/columns/items/type",keyword:"type",params:{type: "string"},message:"must be string"};
if(vErrors === null){
vErrors = [err11];
}
else {
vErrors.push(err11);
}
errors++;
}
var valid11 = _errs38 === errors;
if(!valid11){
break;
}
}
}
else {
const err12 = {instancePath:instancePath+"/result/selection/columns",schemaPath:"#/$defs/TrashSelection/oneOf/2/properties/columns/type",keyword:"type",params:{type: "array"},message:"must be array"};
if(vErrors === null){
vErrors = [err12];
}
else {
vErrors.push(err12);
}
errors++;
}
}
var valid9 = _errs36 === errors;
}
else {
var valid9 = true;
}
if(valid9){
if(data9.kind !== undefined){
let data16 = data9.kind;
const _errs40 = errors;
if(typeof data16 !== "string"){
const err13 = {instancePath:instancePath+"/result/selection/kind",schemaPath:"#/$defs/TrashSelection/oneOf/2/properties/kind/type",keyword:"type",params:{type: "string"},message:"must be string"};
if(vErrors === null){
vErrors = [err13];
}
else {
vErrors.push(err13);
}
errors++;
}
if(!(data16 === "objects")){
const err14 = {instancePath:instancePath+"/result/selection/kind",schemaPath:"#/$defs/TrashSelection/oneOf/2/properties/kind/enum",keyword:"enum",params:{allowedValues: schema215.oneOf[2].properties.kind.enum},message:"must be equal to one of the allowed values"};
if(vErrors === null){
vErrors = [err14];
}
else {
vErrors.push(err14);
}
errors++;
}
var valid9 = _errs40 === errors;
}
else {
var valid9 = true;
}
}
}
}
}
else {
const err15 = {instancePath:instancePath+"/result/selection",schemaPath:"#/$defs/TrashSelection/oneOf/2/type",keyword:"type",params:{type: "object"},message:"must be object"};
if(vErrors === null){
vErrors = [err15];
}
else {
vErrors.push(err15);
}
errors++;
}
}
var _valid0 = _errs30 === errors;
if(_valid0 && valid6){
valid6 = false;
passing0 = [passing0, 2];
}
else {
if(_valid0){
valid6 = true;
passing0 = 2;
if(props0 !== true){
props0 = props0 || {};
props0.cards = true;
props0.columns = true;
props0.kind = true;
}
}
}
}
if(!valid6){
const err16 = {instancePath:instancePath+"/result/selection",schemaPath:"#/$defs/TrashSelection/oneOf",keyword:"oneOf",params:{passingSchemas: passing0},message:"must match exactly one schema in oneOf"};
if(vErrors === null){
vErrors = [err16];
}
else {
vErrors.push(err16);
}
errors++;
validate222.errors = vErrors;
return false;
}
else {
errors = _errs21;
if(vErrors !== null){
if(_errs21){
vErrors.length = _errs21;
}
else {
vErrors = null;
}
}
}
var valid1 = _errs19 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.space !== undefined){
const _errs42 = errors;
if(typeof data1.space !== "string"){
validate222.errors = [{instancePath:instancePath+"/result/space",schemaPath:"#/properties/result/properties/space/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
var valid1 = _errs42 === errors;
}
else {
var valid1 = true;
}
if(valid1){
if(data1.version !== undefined){
let data18 = data1.version;
const _errs44 = errors;
const _errs45 = errors;
if(errors === _errs45){
if(Array.isArray(data18)){
if(data18.length > 32){
validate222.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/maxItems",keyword:"maxItems",params:{limit: 32},message:"must NOT have more than 32 items"}];
return false;
}
else {
if(data18.length < 32){
validate222.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/minItems",keyword:"minItems",params:{limit: 32},message:"must NOT have fewer than 32 items"}];
return false;
}
else {
var valid13 = true;
const len5 = data18.length;
for(let i5=0; i5<len5; i5++){
let data19 = data18[i5];
const _errs47 = errors;
if(!((typeof data19 == "number") && (!(data19 % 1) && !isNaN(data19)))){
validate222.errors = [{instancePath:instancePath+"/result/version/" + i5,schemaPath:"#/$defs/StateVersion/items/type",keyword:"type",params:{type: "integer"},message:"must be integer"}];
return false;
}
if(errors === _errs47){
if(typeof data19 == "number"){
if(data19 > 255 || isNaN(data19)){
validate222.errors = [{instancePath:instancePath+"/result/version/" + i5,schemaPath:"#/$defs/StateVersion/items/maximum",keyword:"maximum",params:{comparison: "<=", limit: 255},message:"must be <= 255"}];
return false;
}
else {
if(data19 < 0 || isNaN(data19)){
validate222.errors = [{instancePath:instancePath+"/result/version/" + i5,schemaPath:"#/$defs/StateVersion/items/minimum",keyword:"minimum",params:{comparison: ">=", limit: 0},message:"must be >= 0"}];
return false;
}
}
}
}
var valid13 = _errs47 === errors;
if(!valid13){
break;
}
}
}
}
}
else {
validate222.errors = [{instancePath:instancePath+"/result/version",schemaPath:"#/$defs/StateVersion/type",keyword:"type",params:{type: "array"},message:"must be array"}];
return false;
}
}
var valid1 = _errs44 === errors;
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
else {
validate222.errors = [{instancePath:instancePath+"/result",schemaPath:"#/properties/result/type",keyword:"type",params:{type: "object"},message:"must be object"}];
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
let data20 = data.status;
const _errs49 = errors;
if(typeof data20 !== "string"){
validate222.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/type",keyword:"type",params:{type: "string"},message:"must be string"}];
return false;
}
if(!(data20 === "ok")){
validate222.errors = [{instancePath:instancePath+"/status",schemaPath:"#/$defs/SuccessStatus/enum",keyword:"enum",params:{allowedValues: schema44.enum},message:"must be equal to one of the allowed values"}];
return false;
}
var valid0 = _errs49 === errors;
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
validate222.errors = [{instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"}];
return false;
}
}
validate222.errors = vErrors;
return errors === 0;
}
validate222.evaluated = {"props":true,"dynamicProps":false,"dynamicItems":false};


function validate221(data, {instancePath="", parentData, parentDataProperty, rootData=data, dynamicAnchors={}}={}){
let vErrors = null;
let errors = 0;
const evaluated0 = validate221.evaluated;
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
if(!(validate222(data, {instancePath,parentData,parentDataProperty,rootData,dynamicAnchors}))){
vErrors = vErrors === null ? validate222.errors : vErrors.concat(validate222.errors);
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
validate221.errors = vErrors;
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
validate221.errors = vErrors;
evaluated0.props = props0;
return errors === 0;
}
validate221.evaluated = {"dynamicProps":true,"dynamicItems":false};

const responseValidators = Object.freeze({
"access.inspect": CommandResponse_AccessSnapshot,
"access.issue": CommandResponse_AccessSnapshot,
"access.list": CommandResponse_AccessList,
"access.replace": CommandResponse_AccessSnapshot,
"access.revoke": CommandResponse_AccessSnapshot,
"backups.catalog": CommandResponse_BackupCatalog,
"backups.catalog.delete": CommandResponse_BackupDeleted,
"backups.catalog.read": CommandResponse_SpaceArchive,
"backups.folder.forget": CommandResponse_BackupFolders,
"backups.folders": CommandResponse_BackupFolders,
"backups.location.change": CommandResponse_BackupLocation,
"backups.location.cleanup": CommandResponse_BackupLocation,
"backups.location.inspect": CommandResponse_BackupLocation,
"card.autosave": CommandResponse_CardSnapshot,
"card.begin_editing": CommandResponse_CardSnapshot,
"card.create": CommandResponse_CardSnapshot,
"card.edit": CommandResponse_CardSnapshot,
"card.finish_editing": CommandResponse_CardSnapshot,
"card.history": CommandResponse_CardHistory,
"card.inspect_placements": CommandResponse_CardPlacements,
"card.inspect_tags": CommandResponse_CardTags,
"card.list": CommandResponse_CardList,
"card.move": CommandResponse_CardSnapshot,
"card.read": CommandResponse_CardSnapshot,
"card.resolve_deletion": CommandResponse_CardSnapshot,
"card.resolve_placement": CommandResponse_CardSnapshot,
"card.resolve_tag": CommandResponse_CardSnapshot,
"card.resolve_text": CommandResponse_CardSnapshot,
"card.restore": CommandResponse_CardSnapshot,
"card.restore_text": CommandResponse_CardSnapshot,
"card.search": CommandResponse_SearchResults,
"card.set_completed": CommandResponse_CardSnapshot,
"card.set_tag": CommandResponse_CardSnapshot,
"card.trash": CommandResponse_CardSnapshot,
"column.create": CommandResponse_ColumnSnapshot,
"column.inspect_positions": CommandResponse_ColumnPositions,
"column.list": CommandResponse_ColumnList,
"column.move": CommandResponse_ColumnSnapshot,
"column.read": CommandResponse_ColumnSnapshot,
"column.rename": CommandResponse_ColumnSnapshot,
"column.resolve_deletion": CommandResponse_ColumnSnapshot,
"column.resolve_name": CommandResponse_ColumnSnapshot,
"column.resolve_position": CommandResponse_ColumnSnapshot,
"column.restore": CommandResponse_ColumnSnapshot,
"column.trash": CommandResponse_ColumnSnapshot,
"history.clear": CommandResponse_HistoryPreview,
"history.inspect": CommandResponse_HistoryPreview,
"invitation.cancel": CommandResponse_InvitationSnapshot,
"invitation.create": CommandResponse_InvitationSnapshot,
"invitation.inspect": CommandResponse_InvitationSnapshot,
"invitation.list": CommandResponse_InvitationList,
"maintenance.execute": CommandResponse_MaintenanceView,
"pairing.cancel": CommandResponse_PairingSnapshot,
"pairing.confirm": CommandResponse_PairingSnapshot,
"pairing.inspect": CommandResponse_PairingSnapshot,
"pairing.list": CommandResponse_PairingList,
"pairing.reject": CommandResponse_PairingSnapshot,
"pairing.unlink": CommandResponse_PairingSnapshot,
"space.backups.delete": CommandResponse_DailyBackups,
"space.backups.inspect": CommandResponse_DailyBackups,
"space.backups.read": CommandResponse_SpaceArchive,
"space.backups.retry": CommandResponse_DailyBackups,
"space.create": CommandResponse_SpaceSnapshot,
"space.export": CommandResponse_SpaceArchive,
"space.import": CommandResponse_ImportedSpace,
"space.list": CommandResponse_SpaceList,
"space.read": CommandResponse_SpaceSnapshot,
"space.recovery.inspect": CommandResponse_RecoveryState,
"space.recovery.inspect_missing": CommandResponse_MissingRecoveryState,
"space.recovery.pause": CommandResponse_RecoveryState,
"space.recovery.read_backup": CommandResponse_SpaceArchive,
"space.recovery.restore": CommandResponse_RestoredSnapshot,
"space.recovery.restore_missing": CommandResponse_ImportedSpace,
"space.rename": CommandResponse_SpaceSnapshot,
"space.resolve_deletion": CommandResponse_SpaceSnapshot,
"space.resolve_name": CommandResponse_SpaceSnapshot,
"space.restore": CommandResponse_SpaceSnapshot,
"space.trash": CommandResponse_SpaceSnapshot,
"sync.catalog": CommandResponse_SyncCatalog,
"sync.inspect": CommandResponse_SyncControl,
"sync.pause": CommandResponse_SyncControl,
"sync.pull": CommandResponse_SyncPage,
"sync.push": CommandResponse_SyncPage,
"sync.select": CommandResponse_SyncControl,
"tag.create": CommandResponse_TagSnapshot,
"tag.delete": CommandResponse_TagSnapshot,
"tag.list": CommandResponse_TagList,
"tag.read": CommandResponse_TagSnapshot,
"tag.rename": CommandResponse_TagSnapshot,
"tag.resolve_deletion": CommandResponse_TagSnapshot,
"tag.resolve_names": CommandResponse_TagList,
"trash.inspect": CommandResponse_TrashPreview,
"trash.purge": CommandResponse_TrashPreview,
"trash.resolve_purge": CommandResponse_TrashPreview
});
export const operations = Object.freeze(Object.keys(responseValidators));
export default function validate(value, operation) {
  if (operation !== undefined) return Object.hasOwn(responseValidators, operation) && responseValidators[operation](value);
  return Object.values(responseValidators).some(check => check(value));
}
