import sys,struct
d=open(sys.argv[1],'rb').read()
sc,mc,so,mo,bv=struct.unpack_from('<5i',d,0)
print(f'len={len(d)} StartMarkerCount={sc} MarkerCount={mc} StartMarkerOffset={so:#x} MarkerOffset={mo:#x} BaseVolume={bv}')
print(' extra header bytes:',d[20:min(so,mo)].hex() if min(so,mo)>20 else '')
print('START MARKERS: idx Name Pos MType Flags Extra LoopStart MCount LoopMCount | MarkerPos IsInstant InstantBuffer State0 State1')
for i in range(sc):
    v=struct.unpack_from('<13i',d,so+i*52)
    print(' S%2d'%i,v[:8],'|',v[8:])
print('MARKERS: idx Name Pos MType Flags Extra LoopStart MarkerCount LoopMarkerCount')
for i in range(mc):
    v=struct.unpack_from('<8i',d,mo+i*32)
    print(' M%3d'%i,v, 't=%.3fs'%(v[1]/2/16*28/32000))
print('end of tables', max(so+sc*52, mo+mc*32), 'file', len(d))
