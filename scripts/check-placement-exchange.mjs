import assert from "node:assert/strict";
import { randomBytes } from "node:crypto";

export function checkPlacementExchange({ host, android, space, pairing, call, command, ok, error }) {
  const id=()=>randomBytes(16).toString("hex");
  const exchange=()=>ok(call(android,"exchange.run",{space}));
  const pause=paused=>{const current=ok(command(android,"sync.inspect",{space,pairing}));ok(command(android,"sync.pause",{space,pairing,paused,expected:current.pause_version}));};
  for (const same of [false,true]) {
    const column=id(), other=id(), card=id(), a=id(), b=id(), x=id(), y=id();
    const read=(target,current=card)=>ok(command(target,"card.read",{space,card:current}));
    const history=target=>ok(command(target,"card.history",{space,card,after:null,limit:100,expected:null})).versions;
    const move=(target,current,destination,before)=>ok(command(target,"card.move",{space,card:current,column:destination,before,expected:read(target,current).version}));
    const choose=(target,index)=>{const current=ok(command(target,"card.inspect_placements",{space,card}));return ok(command(target,"card.resolve_placement",{space,card,expected:current.card.version,placement:current.choices[index].placement}));};
    for (const current of [column,other]) ok(command(host,"column.create",{space,column:current,name:`Placement ${current}`}));
    for (const current of [a,b,card,x,y]) ok(command(host,"card.create",{space,column,card:current,title:`Saved ${current}`,description:"Placement body"}));
    const tag=id(), created=ok(command(host,"tag.create",{space,tag,name:`Placement ${tag}`}));
    ok(command(host,"card.set_tag",{space,card,tag,present:true,expected:read(host).version,expected_tag:created.version}));
    ok(command(host,"card.set_completed",{space,card,completed:true,expected:read(host).version}));
    const base=read(host), versions=history(host);
    exchange(); pause(true);
    move(host,x,column,b); move(android,y,column,b);
    move(host,card,column,b); move(android,card,same?column:other,same?a:null);
    pause(false); exchange(); exchange();
    const conflicted=read(host); assert.equal(conflicted.state,"conflicted"); assert.equal(conflicted.placements.length,2);
    assert.deepEqual(read(android),conflicted);
    const list=target=>ok(command(target,"card.list",{space,include_trashed:false})).cards;
    assert.deepEqual(list(host),list(android));
    const order=list(host).filter(item=>item.columns.length===1&&item.positions.length===1&&item.card!==card).map(item=>item.card);
    assert.ok(order.indexOf(a)<order.indexOf(x)&&order.indexOf(x)<order.indexOf(b));
    assert.ok(order.indexOf(a)<order.indexOf(y)&&order.indexOf(y)<order.indexOf(b));
    assert.equal(read(host,x).state,"active"); assert.equal(read(host,y).state,"active");
    error(command(android,"card.set_completed",{space,card,completed:false,expected:conflicted.version}),"conflict");
    error(command(android,"card.move",{space,card,column,before:null,expected:conflicted.version}),"conflict");
    pause(true); choose(host,0); choose(android,1); pause(false); exchange(); exchange();
    const again=read(android); assert.equal(again.placements.length,2); assert.deepEqual(again.placements,conflicted.placements);
    const resolved=choose(android,0); exchange(); exchange();
    assert.equal(resolved.state,"active"); assert.deepEqual(read(host),read(android));
    for (const key of ["card","titles","descriptions","tags","completed","deletion_groups"]) assert.deepEqual(read(host)[key],base[key]);
    assert.deepEqual(history(host),versions); assert.deepEqual(history(android),versions);
    move(android,card,column,b); exchange();
    assert.deepEqual(read(host).columns,[column]); assert.equal(read(host).state,"active");
    assert.deepEqual(history(host),versions);
  }
  console.log("Card placements: cross/same-column conflicts, whole choices, independent decisions, same neighbors, preservation and reverse passed.");
}
