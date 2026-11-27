import contextlib
from email import policy
from email.parser import BytesParser
import hashlib
import io
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import zipfile

import deliver_collected_paper as delivery


class DeliveryTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.repo = Path(self.temp.name)
        (self.repo / 'manuscript').mkdir()
        (self.repo / 'manuscript/z.tex').write_text('Z')
        (self.repo / 'manuscript/main.tex').write_text('A source')
        (self.repo / 'paper.pdf').write_bytes(b'%PDF-test')
        self.envelope = {
            'schema': delivery.SCHEMA, 'producer': 'internal', 'work_id': 'protocol',
            'version': 1, 'previous_version': None, 'authorize_publication': True,
            'paper': {'title': 'Protocol', 'abstract': 'A checked mechanism.', 'authors': [{'name': 'Author'}], 'license': 'CC-BY-4.0', 'kind': 'methodology', 'msc': [], 'repository_url': 'https://github.com/mathesis-papers/protocol'},
            'review': {'verdict': 'INTERNAL_EDITORIAL', 'record': {'editorial': 'Checked by the operator'}},
        }
        self.save()

    def save(self):
        (self.repo / 'mathesis.json').write_text(json.dumps(self.envelope))

    def test_deterministic_zip_ignores_mtime_permissions_and_creation_order(self):
        first = delivery.package(self.repo)
        os.utime(self.repo / 'manuscript/main.tex', (1700000000, 1700000000))
        (self.repo / 'manuscript/main.tex').chmod(0o755)
        second = delivery.package(self.repo)
        self.assertEqual(first, second)
        envelope, pdf, source = first
        self.assertEqual(envelope['review']['pdf_sha256'], hashlib.sha256(pdf).hexdigest())
        self.assertEqual(envelope['review']['source_sha256'], hashlib.sha256(source).hexdigest())
        with zipfile.ZipFile(io.BytesIO(source)) as archive:
            self.assertEqual(archive.namelist(), ['main.tex', 'z.tex'])
            self.assertEqual(archive.read('main.tex'), b'A source')
            self.assertEqual(archive.getinfo('main.tex').date_time, (1980, 1, 1, 0, 0, 0))

    def test_legacy_pass_envelope_keeps_optional_fields_absent(self):
        del self.envelope['paper']['kind']
        del self.envelope['paper']['repository_url']
        self.envelope['review']['verdict'] = 'PASS'
        self.save()
        envelope, _, _ = delivery.package(self.repo)
        for key in ('kind', 'repository_url', 'issues_url', 'version_label', 'changelog'):
            self.assertNotIn(key, envelope['paper'])
        self.assertEqual(envelope['review']['verdict'], 'PASS')

    def test_dry_run_is_offline_emits_valid_envelope_and_no_token(self):
        with patch.object(delivery, 'request', side_effect=AssertionError('network')), patch.dict(os.environ, {'MATHESIS_TOKEN': 'secret-sentinel'}), contextlib.redirect_stdout(io.StringIO()) as out:
            self.assertEqual(delivery.main([str(self.repo), '--dry-run', '--version-label', 'v1.0', '--changelog', 'Initial version']), 0)
        result = json.loads(out.getvalue())
        self.assertEqual(result['paper']['version_label'], 'v1.0')
        self.assertNotIn('secret-sentinel', out.getvalue())
        delivery.validate(result)

    def test_post_multipart_preserves_exact_bytes_and_fields(self):
        envelope, pdf, source = delivery.package(self.repo)
        body, content_type = delivery.multipart(envelope, pdf, source)
        message = BytesParser(policy=policy.default).parsebytes(f'Content-Type: {content_type}\r\nMIME-Version: 1.0\r\n\r\n'.encode() + body)
        fields = {part.get_param('name', header='content-disposition'): part for part in message.iter_parts()}
        self.assertEqual(set(fields), {'metadata', 'pdf', 'source'})
        self.assertEqual(fields['pdf'].get_payload(decode=True), pdf)
        self.assertEqual(fields['source'].get_payload(decode=True), source)
        self.assertEqual(json.loads(fields['metadata'].get_payload(decode=True)), envelope)
        with patch.dict(os.environ, {'MATHESIS_URL': 'https://mathesis.example', 'MATHESIS_TOKEN': 'test-token'}), patch.object(delivery, 'request', return_value={'status': 'collected'}) as post, contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(delivery.main([str(self.repo)]), 0)
            self.assertEqual(post.call_args.args[0], 'https://mathesis.example/api/v1/pre-reviewed-deliveries')
            self.assertEqual(post.call_args.kwargs['token'], 'test-token')
            self.assertEqual(post.call_args.kwargs['data'], body)

    def test_predecessor_lookup_handles_new_successor_retry_and_stale(self):
        self.envelope['version'] = 2
        for result, expected in [(None, None), ({'revision': '1'}, 1), ({'revision': '2', 'previous_version': '1'}, 1)]:
            with patch.object(delivery, 'request', return_value=result):
                self.assertEqual(delivery.previous_version(self.envelope, 'https://mathesis.example/api/v1'), expected)
        with patch.object(delivery, 'request', return_value={'revision': '3'}):
            with self.assertRaises(ValueError):
                delivery.previous_version(self.envelope, 'https://mathesis.example/api/v1')

    def test_rejects_invalid_metadata_and_symlinks(self):
        for key, value in [('kind', 'conjecture'), ('version_label', 'x' * 65), ('changelog', 'x' * 10001), ('repository_url', 'https://github.com/a/b?x'), ('repository_url', 'https://github.com/a/..'), ('issues_url', 'https://github.com/other/repo/issues')]:
            with self.subTest(key=key, value=value[:50]):
                invalid = dict(self.envelope['paper'])
                invalid[key] = value
                self.envelope['paper'], original = invalid, self.envelope['paper']
                self.save()
                with self.assertRaises(ValueError):
                    delivery.package(self.repo)
                self.envelope['paper'] = original
        self.save()
        (self.repo / 'manuscript/escape.tex').symlink_to(self.repo / 'paper.pdf')
        with self.assertRaises(ValueError):
            delivery.package(self.repo)

    def test_rejects_redirects_and_unsafe_api_targets(self):
        self.assertIsNone(delivery.NoRedirect().redirect_request(None, None, 302, '', {}, 'https://other.example'))
        for url in ['http://remote.example', 'https://token@mathesis.example', 'https://mathesis.example?token=x']:
            with self.assertRaises(ValueError):
                delivery.api_base(url)
        self.assertEqual(delivery.api_base('http://127.0.0.1:8000'), 'http://127.0.0.1:8000/api/v1')


if __name__ == '__main__':
    unittest.main()
