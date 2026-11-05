#!/usr/bin/env python3
"""Prepare a development-only signing identity from a secret, cache or fresh key."""
import argparse
import base64
import os
from pathlib import Path
import subprocess

def prepare(path, keytool, configured=''):
    path=Path(path)
    previous=os.umask(0o077)
    try:
        path.parent.mkdir(parents=True,exist_ok=True)
        if configured:
            # Decode before touching an existing cache file.
            value=base64.b64decode(configured,validate=True)
            if not value:
                raise ValueError('The configured development keystore is empty')
            partial=path.with_suffix('.partial')
            partial.write_bytes(value)
            partial.chmod(0o600)
            partial.replace(path)
            source='secret'
        elif path.is_file():
            source='cache'
        else:
            subprocess.run([str(keytool),'-genkeypair','-keystore',str(path),
                '-storepass','android','-keypass','android','-alias','androiddebugkey',
                '-keyalg','RSA','-validity','10000','-dname','CN=Horizon Android Development'],
                check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
            source='generated'
        path.chmod(0o600)
        # Fail instead of silently rotating an invalid cached/configured identity.
        subprocess.run([str(keytool),'-list','-keystore',str(path),
            '-storepass','android','-alias','androiddebugkey'],
            check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
        return source
    finally:
        os.umask(previous)

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--keytool',default='keytool')
    parser.add_argument('--keystore',type=Path,default=Path('.local/android/debug.keystore'))
    args=parser.parse_args()
    try:
        source=prepare(args.keystore,args.keytool,os.environ.get('DEV_KEYSTORE',''))
    except (ValueError,OSError,subprocess.CalledProcessError) as error:
        raise SystemExit('Could not prepare/validate the development signing key; check its base64 value, password, alias or cached file.') from error
    print(f'Development signing identity: {source}')
    if output:=os.environ.get('GITHUB_OUTPUT'):
        with open(output,'a',encoding='utf-8') as handle:
            handle.write(f'source={source}\n')
if __name__=='__main__':main()
