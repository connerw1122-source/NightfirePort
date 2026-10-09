import math
def tdiv(a,b):
    q=abs(a)//abs(b); return q if (a<0)==(b<0) else -q
def wrap64(v):
    v&=(1<<64)-1; return v-(1<<64) if v>>63 else v
def ps2sqrt(n):
    n=n&0xffffffff
    if n==0: return 0
    ns=n-(1<<32) if n>>31 else n
    x=1
    for _ in range(10):
        x=wrap64(x-tdiv(wrap64(x*x-ns),wrap64(2*x)))
    return x&0xffffffff
for n in [1,2,4,10,100,1000,9999,10000,40000,100000,250000,1000000,4000000,10**7,10**8,10**9]:
    r=ps2sqrt(n); print(n, r, round(math.sqrt(n),2))
# first n where result deviates from floor/ceil sqrt by >1
bad=None
for n in range(1,3000000,7):
    r=ps2sqrt(n); s=math.isqrt(n)
    if abs(r-s)>1: bad=n;break
print('first n with |err|>1:',bad, bad and (ps2sqrt(bad),math.isqrt(bad)))
# shuffle
import itertools
def shuffle_order(N, rs):
    used=[0]*50; rem=N; out=[]
    for k in range(N):
        r=rs[k]; idx=0
        while True:
            if r>0: idx+=1
            while used[idx]: idx+=1
            c=r>0; r-=1
            if not c: break
        used[idx]=1; rem-=1; out.append(idx)
    return out
for N in range(1,12):
    bad=set()
    for rs in itertools.product(*[range(N-k) for k in range(N)]) if N<=7 else []:
        o=shuffle_order(N,rs)
        if max(o)>=N or len(set(o))!=N: bad.add(tuple(o))
    print('N',N,'bad perms',len(bad), list(bad)[:3])
