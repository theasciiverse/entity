"""Tiny symbolic decompiler for ActionScript 1/2 bytecode (SWF DoAction bodies).
Prints straight-line pseudo code with gotos; good enough to read game logic."""
import os,struct,sys
sys.path.insert(0,os.path.dirname(os.path.abspath(__file__)))
import swf

BIN={0x0a:'+',0x0b:'-',0x0c:'*',0x0d:'/',0x47:'+',0x3f:'%',0x60:'&',0x61:'|',0x62:'^',0x63:'<<',0x64:'>>',0x65:'>>>',
     0x49:'==',0x0e:'==',0x48:'<',0x0f:'<',0x67:'>',0x66:'===',0x10:'&&',0x11:'||',0x21:'+',0x13:'==',0x29:'<',0x68:'>'}
def num(x):
    if isinstance(x,float):
        r=round(x,6)
        return str(int(r)) if r==int(r) else repr(r)
    return str(x)

def decompile(a, lo=0, hi=None, indent=0, pool0=None, out=None, regs=None):
    """walk actions in a[lo:hi]; returns lines"""
    hi=len(a) if hi is None else hi
    pool=list(pool0 or [])
    lines=[] if out is None else out
    st=[]
    i=lo
    ind=lambda n=0:'  '*(indent+n)
    targets=set()
    # pre-scan jump targets for labels
    j=lo
    while j<hi:
        op=a[j]; j+=1
        if op>=0x80:
            ln=struct.unpack('<H',a[j:j+2])[0]; j+=2
            if op in (0x99,0x9d):
                off=struct.unpack('<h',a[j:j+2])[0]; targets.add(j+ln+off)
            if op in (0x9b,):
                pass
            j+=ln
    fnstack=[]  # (end offset)
    def pop():
        return st.pop() if st else '?'
    while i<hi:
        while fnstack and i>=fnstack[-1]:
            fnstack.pop(); lines.append(ind(len(fnstack))+'}')
        if i in targets: lines.append(f'{ind(len(fnstack)-1 if fnstack else 0)}L{i}:')
        op=a[i]; at=i; i+=1
        d=ind(len(fnstack))
        if op<0x80:
            if op in BIN:
                b=pop(); x=pop(); st.append(f'({x} {BIN[op]} {b})')
            elif op==0x12: st.append(f'!{pop()}')
            elif op==0x4e: n=pop(); o=pop(); st.append(f'{o}.{n}' if isinstance(n,str) and n.isidentifier() else f'{o}[{n}]')
            elif op==0x4f:
                v=pop(); n=pop(); o=pop(); lines.append(f'{d}{o}.{n} = {v};')
            elif op==0x1c: n=pop(); st.append(n)
            elif op==0x1d: v=pop(); n=pop(); lines.append(f'{d}{n} = {v};')
            elif op==0x3c: v=pop(); n=pop(); lines.append(f'{d}var {n} = {v};')
            elif op==0x41: n=pop(); lines.append(f'{d}var {n};'); 
            elif op==0x17: lines.append(f'{d}{pop()};')
            elif op==0x3e: lines.append(f'{d}return {pop()};')
            elif op==0x4c: x=pop(); st.append(x); st.append(x)
            elif op==0x4d: x=pop(); y=pop(); st.append(x); st.append(y)
            elif op==0x50: x=pop(); st.append(f'({x}+1)')
            elif op==0x51: x=pop(); st.append(f'({x}-1)')
            elif op==0x4a: st.append(f'Number({pop()})')
            elif op==0x4b: st.append(f'String({pop()})')
            elif op==0x18: st.append(f'int({pop()})')
            elif op==0x44: st.append(f'typeof {pop()}')
            elif op==0x52 or op==0x53:
                m=pop(); o=pop(); n=int(float(pop())) if str(st[-1]).lstrip('-').replace('.','').isdigit() else 0
                args=[pop() for _ in range(n)]
                call=f'{o}.{m}({", ".join(args)})' if str(m)!='undefined' else f'{o}({", ".join(args)})'
                st.append(call)
            elif op==0x3d:
                f=pop(); n=int(float(pop())) if str(st[-1]).lstrip('-').replace('.','').isdigit() else 0
                args=[pop() for _ in range(n)]
                st.append(f'{f}({", ".join(args)})')
            elif op==0x40:
                f=pop(); n=int(float(pop())) if str(st[-1]).lstrip('-').replace('.','').isdigit() else 0
                args=[pop() for _ in range(n)]
                st.append(f'new {f}({", ".join(args)})')
            elif op==0x42:
                n=int(float(pop())); st.append('['+', '.join(pop() for _ in range(n))+']')
            elif op==0x43:
                n=int(float(pop())); items=[]
                for _ in range(n): v=pop(); k=pop(); items.append(f'{k}: {v}')
                st.append('{'+', '.join(items)+'}')
            elif op==0x30: st.append(f'random({pop()})')
            elif op==0x3a or op==0x3b: n=pop(); lines.append(f'{d}delete {n};')
            elif op==0x35 or op==0x34: st.append('getTime()')
            else: lines.append(f'{d}// op 0x{op:02x}')
        else:
            ln=struct.unpack('<H',a[i:i+2])[0]; i+=2; body=a[i:i+ln]
            if op==0x88:
                n=struct.unpack('<H',body[:2])[0]; pool=[s.decode('latin1') for s in body[2:].split(b'\x00')[:n]]
            elif op==0x96:
                j=0
                while j<ln:
                    t=body[j]; j+=1
                    if t==0: k=body.index(b'\x00',j); st.append(repr(body[j:k].decode('latin1'))[1:-1] if False else body[j:k].decode('latin1')); j=k+1
                    elif t==1: st.append(num(struct.unpack('<f',body[j:j+4])[0])); j+=4
                    elif t==2: st.append('null')
                    elif t==3: st.append('undefined')
                    elif t==4: st.append(f'R{body[j]}'); j+=1
                    elif t==5: st.append('true' if body[j] else 'false'); j+=1
                    elif t==6: st.append(num(struct.unpack('<d',body[j+4:j+8]+body[j:j+4])[0])); j+=8
                    elif t==7: st.append(str(struct.unpack('<i',body[j:j+4])[0])); j+=4
                    elif t==8: st.append(pool[body[j]] if body[j]<len(pool) else f'C{body[j]}'); j+=1
                    elif t==9:
                        x=struct.unpack('<H',body[j:j+2])[0]; st.append(pool[x] if x<len(pool) else f'C{x}'); j+=2
            elif op==0x87:
                x=pop(); lines.append(f'{d}R{body[0]} = {x};'); st.append(f'R{body[0]}')
            elif op==0x9d:
                off=struct.unpack('<h',body[:2])[0]; c=pop(); lines.append(f'{d}if ({c}) goto L{i+ln+off};')
            elif op==0x99:
                off=struct.unpack('<h',body[:2])[0]; lines.append(f'{d}goto L{i+ln+off};')
            elif op==0x9b:
                k=body.index(b'\x00'); name=body[:k].decode('latin1'); n=struct.unpack('<H',body[k+1:k+3])[0]
                p=k+3; params=[]
                for _ in range(n):
                    q=body.index(b'\x00',p); params.append(body[p:q].decode('latin1')); p=q+1
                size=struct.unpack('<H',body[p:p+2])[0]
                lines.append(f'{d}function {name}({", ".join(params)}) {{')
                fnstack.append(i+ln+size)
                if name: pass
                else: st.append('function')
            elif op==0x8e:
                k=body.index(b'\x00'); name=body[:k].decode('latin1'); n=struct.unpack('<H',body[k+1:k+3])[0]
                nregs=body[k+3]; flags=struct.unpack('<H',body[k+4:k+6])[0]; p=k+6; params=[]
                for _ in range(n):
                    reg=body[p]; q=body.index(b'\x00',p+1); params.append(f'{body[p+1:q].decode("latin1")}@R{reg}'); p=q+1
                size=struct.unpack('<H',body[p:p+2])[0]
                lines.append(f'{d}function {name}({", ".join(params)}) {{ // regs={nregs}')
                fnstack.append(i+ln+size)
            else:
                lines.append(f'{d}// action 0x{op:02x} {body[:16]!r}')
            i+=ln
    while fnstack: fnstack.pop(); lines.append('}')
    for x in st: lines.append(f'// leftover on stack: {x}')
    return lines

if __name__=='__main__':
    import sys
    tags=swf.load(sys.argv[1])
    idx=int(sys.argv[2])
    a=tags[idx][1]
    print('\n'.join(decompile(a)))
