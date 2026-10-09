import struct, sys, collections, os
# Bank files extracted with `nf-extract iso-get` into work/audio/ (never committed).
A=os.path.join(os.path.dirname(os.path.abspath(__file__)),'..','..','work','audio','')
sfx=open(A+'SB_4.SFX','rb').read(); shf=open(A+'SB_4.SHF','rb').read()
nsamp=struct.unpack_from('<I',shf,0)[0]
hdr=[struct.unpack_from('<9i',shf,4+i*36) for i in range(nsamp)]
n=struct.unpack_from('<I',sfx,0)[0]
F=['ReverbSend','TrackingType','Inner','Outer','MaxVoices','Priority','Group']
S=['MaxReject','Action2','IgnoreAge','Ducker']
def parse(off):
    v=dict(zip(F,struct.unpack_from('<7i',sfx,off)))
    v.update(zip(S,struct.unpack_from('<4h',sfx,off+0x1c)))
    v['DuckerLength'],v['MasterVolume'],v['MinDelay'],v['MaxDelay']=struct.unpack_from('<4i',sfx,off+0x24)
    v.update(zip(['MultiSample','RandomPick','Shuffled','Loop','Polyphonic'],struct.unpack_from('<5h',sfx,off+0x34)))
    v['Outdoors'],v['PauseInNis']=struct.unpack_from('<2b',sfx,off+0x3e)
    v['SampleCount']=struct.unpack_from('<i',sfx,off+0x40)[0]
    v['subs']=[struct.unpack_from('<7i',sfx,off+0x44+k*28) for k in range(v['SampleCount'])]
    return v
ents=[]
for i in range(n):
    h,o=struct.unpack_from('<Ii',sfx,4+i*8)
    ents.append((h,o,parse(o)))
print('sfx count',n,'file size',len(sfx),'samples',nsamp)
# contiguity check
offs=sorted(o for _,o,_ in ents)
end=[o+0x44+28*e['SampleCount'] for _,o,e in sorted(ents,key=lambda x:x[1])]
gaps=[(offs[i+1]-end[i]) for i in range(len(offs)-1)]
print('first block offset',offs[0],'expected',4+8*n,'gaps set',set(gaps),'last end',end[-1],'== size',end[-1]==len(sfx))
tot=sum(e['SampleCount'] for _,_,e in ents); print('total subtracks',tot)
for k in list(F)+S+['DuckerLength','MasterVolume','MinDelay','MaxDelay','MultiSample','RandomPick','Shuffled','Loop','Polyphonic','Outdoors','PauseInNis','SampleCount']:
    c=collections.Counter(e[k] for _,_,e in ents)
    print(f'{k:13s}', dict(sorted(c.items())) if len(c)<=14 else (min(c),max(c),'distinct',len(c)))
subs=[s for _,_,e in ents for s in e['subs']]
names=['FileRef','PitchOffset','RandomPitchOffset','BaseVolume','RandomVolumeOffset','Pan','RandomPan']
for j,nm in enumerate(names):
    c=collections.Counter(s[j] for s in subs)
    print(f'{nm:18s}', dict(sorted(c.items())) if len(c)<=16 else (min(c),max(c),'distinct',len(c)))
neg=[s[0] for s in subs if s[0]<0]; print('negative filerefs',neg[:20],len(neg))
used=set(s[0] for s in subs if s[0]>=0); print('distinct sample refs',len(used),'max',max(used),'unreferenced',nsamp-len(used))
print('loop-flag samples', sum(1 for h in hdr if h[0]&1), 'flags set', collections.Counter(h[0] for h in hdr))
print('pitch', collections.Counter(h[3] for h in hdr).most_common(8))
# combos
print('combos (Multi,Shuf,Loop,Poly,count>1):',collections.Counter((e['MultiSample'],e['Shuffled'],e['Loop'],e['Polyphonic'],e['SampleCount']>1) for _,_,e in ents))
print('looping sample used by Loop param:',collections.Counter((e['Loop'], any(hdr[s[0]][0]&1 for s in e['subs'] if s[0]>=0)) for _,_,e in ents))
print('poly counts', [ (h,e['SampleCount']) for h,_,e in ents if e['Polyphonic']])
for want in map(int,sys.argv[1:]):
    for h,o,e in ents:
        if h==want:
            print('\n== id',h,'off',hex(o)); 
            for k,v in e.items():
                if k!='subs': print(' ',k,v)
            for s in e['subs']:
                hh=hdr[s[0]] if 0<=s[0]<nsamp else None
                print('   sub',dict(zip(names,s)),'-> shf',hh and dict(flags=hh[0],size=hh[2],pitch=hex(hh[3])))
print('\n-- trailing 28-byte record after each entry')
tc=collections.Counter()
for h,o,e in ents:
    t=struct.unpack_from('<7i',sfx,o+0x44+28*e['SampleCount'])
    tc[t]+=1
    if e['SampleCount']==0: print('count0 id',h,'Tracking',e['TrackingType'],'trail',t)
print(tc.most_common(10))
sbf=open(A+'SB_4.SBF','rb').read()
h0=hdr[0]; print('sample0 hdr',h0, 'data nonzero bytes', sum(1 for b in sbf[h0[1]:h0[1]+h0[2]] if b), sbf[h0[1]:h0[1]+48].hex())
print('hdr fields 0x10,0x1c,0x20 distinct', collections.Counter(h[4]==h[2] for h in hdr), collections.Counter(h[7] for h in hdr).most_common(3), collections.Counter(h[8] for h in hdr).most_common(5))
print('looped samples', [(i,h) for i,h in enumerate(hdr) if h[0]&1])
print('ids using looped samples', [(hh,e['TrackingType'],e['Loop'],[s[0] for s in e['subs']]) for hh,_,e in ents if any(s[0]>=0 and hdr[s[0]][0]&1 for s in e['subs'])])
print('Multi/Poly/Shuffle/Loop entries:')
for hh,o,e in ents:
    if e['MultiSample'] or e['Polyphonic'] or e['Shuffled'] or e['Loop'] or e['MinDelay'] or e['IgnoreAge'] or e['Outdoors'] or e['PauseInNis']:
        print(' ',hh,{k:e[k] for k in ('TrackingType','MultiSample','RandomPick','Shuffled','Loop','Polyphonic','MinDelay','MaxDelay','IgnoreAge','Outdoors','PauseInNis','SampleCount','Priority')},[s[0] for s in e['subs']])
print('Pan nonzero', [(hh,e['TrackingType'],[s[5] for s in e['subs']]) for hh,_,e in ents if any(s[5] for s in e['subs'])])
print('Ducker entries', [(hh,e['TrackingType'],e['Ducker'],e['DuckerLength']) for hh,_,e in ents if e['Ducker']][:40])
print('stream entries', [(hh,e['TrackingType'],e['SampleCount'],[s[0] for s in e['subs']]) for hh,_,e in ents if any(s[0]<0 for s in e['subs'])])
print('Inner>Outer?', [(hh,e['Inner'],e['Outer']) for hh,_,e in ents if e['Inner']>=e['Outer']])
print('Tracking 1/3', [(hh,e['TrackingType'],e['Inner'],e['Outer']) for hh,_,e in ents if e['TrackingType'] in (1,3)])
print('neg pitch offset', [(hh,[ (s[0],s[1]) for s in e['subs'] if s[1]]) for hh,_,e in ents if any(s[1] for s in e['subs'])])
print('SampleCount>10 & poly', [(hh) for hh,_,e in ents if e['Polyphonic'] and e['SampleCount']>10])
