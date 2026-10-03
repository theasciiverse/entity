"""Helpers for reading Metanet N v1.2 (Flash 6) bytecode. Usage: python3 tools/asdec.py Nv12.swf TAG_INDEX
(see CLAUDE.md, "Original N physics"). swf.py parses tags and constant pools; asdec.py decompiles
ActionScript 1/2 bodies into readable pseudo code."""
import struct
def load(path='Nv12.swf'):
    b=open(path,'rb').read()
    if b[:3]==b'CWS':
        import zlib
        b=b'FWS'+b[3:8]+zlib.decompress(b[8:])
    nbits=b[8]>>3
    p=8+(5+4*nbits+7)//8+4
    tags=[]
    while p<len(b):
        h=struct.unpack('<H',b[p:p+2])[0]; p+=2
        t=h>>6; l=h&63
        if l==63: l=struct.unpack('<I',b[p:p+4])[0]; p+=4
        tags.append((t,b[p:p+l])); p+=l
    return tags
def pool(a):
    """first ConstantPool in a DoAction body -> list of bytes strings"""
    i=0
    while i<len(a):
        op=a[i]; i+=1
        if op>=0x80:
            ln=struct.unpack('<H',a[i:i+2])[0]; i+=2
            if op==0x88:
                n=struct.unpack('<H',a[i:i+2])[0]
                parts=a[i+2:i+ln].split(b'\x00')
                return parts[:n]
            i+=ln
    return []

def pushes(a):
    """yield (offset, [values]) for each ActionPush, resolving constant pool refs against the first pool seen"""
    P=pool(a)
    i=0
    while i<len(a):
        op=a[i]; i+=1
        if op<0x80: continue
        ln=struct.unpack('<H',a[i:i+2])[0]; i+=2
        if op==0x88:
            n=struct.unpack('<H',a[i:i+2])[0]
            P=a[i+2:i+ln].split(b'\x00')[:n]
        if op==0x96:
            j=i; end=i+ln; vals=[]
            while j<end:
                t=a[j]; j+=1
                if t==0:
                    k=a.index(b'\x00',j); vals.append(('s',a[j:k].decode('latin1'))); j=k+1
                elif t==1: vals.append(('f',struct.unpack('<f',a[j:j+4])[0])); j+=4
                elif t in(2,3): vals.append(('n',None))
                elif t==4: vals.append(('reg',a[j])); j+=1
                elif t==5: vals.append(('b',a[j])); j+=1
                elif t==6: vals.append(('d',struct.unpack('<d',a[j+4:j+8]+a[j:j+4])[0])); j+=8
                elif t==7: vals.append(('i',struct.unpack('<i',a[j:j+4])[0])); j+=4
                elif t==8:
                    vals.append(('c',P[a[j]].decode('latin1') if a[j]<len(P) else '?')); j+=1
                elif t==9:
                    x=struct.unpack('<H',a[j:j+2])[0]; vals.append(('c',P[x].decode('latin1') if x<len(P) else '?')); j+=2
                else: vals.append(('?',t)); break
            yield i-3,vals
        i+=ln
