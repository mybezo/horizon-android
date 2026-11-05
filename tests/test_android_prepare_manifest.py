"""Android preparation must use metadata accepted by the engine, even after republication."""
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest import mock
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'tools'))
import android_prepare as prepare
class AndroidManifestTests(unittest.TestCase):
    def test_immutable_engine_metadata_is_used_without_live_catalog(self):
        pins={'version':'26.3','manifest_sha1':'a'*40,'client_sha1':'b'*40,'server_sha1':'c'*40}
        manifest={'downloads':{s:{'url':f'https://example.invalid/{s}','sha1':pins[f'{s}_sha1']} for s in ['client','server']},'assetIndex':{'id':'34','url':'https://example.invalid/assets','sha1':'d'*40}}
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder)
            (root/'research-lock.json').write_text(json.dumps({'minecraft':pins}))
            def download(url,path,checksum):
                path.parent.mkdir(parents=True,exist_ok=True)
                path.write_text(json.dumps(manifest if path.name=='26.3.json' else {'objects':{}}))
            with mock.patch.object(prepare,'ROOT',root), mock.patch.object(prepare,'fetch_json',side_effect=AssertionError('Live catalog must not select engine metadata')), mock.patch.object(prepare.lab,'download',side_effect=download) as downloaded, mock.patch.object(prepare.lab,'client_library_artifacts',return_value=[]), mock.patch.object(prepare,'parallel_download'):
                result,server=prepare.download_game('26.3',root/'minecraft')
            self.assertEqual(result,manifest)
            self.assertEqual(downloaded.call_args_list[0].args[0],f'https://piston-meta.mojang.com/v1/packages/{pins["manifest_sha1"]}/26.3.json')
            self.assertEqual(downloaded.call_args_list[0].args[2],pins['manifest_sha1'])
            self.assertEqual(server.name,'26.3-server.jar')
if __name__=='__main__':unittest.main()
