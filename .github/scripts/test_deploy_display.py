import copy
import unittest
from unittest.mock import patch
import deploy_display as d


class RolloutTests(unittest.TestCase):
    def test_source_and_image_receipts_are_bound_to_the_exact_publisher(self):
        receipt = {'source_ref':'develop','source_commit':'a'*40,'workflow_run':'12','image':d.IMAGE+'b'*64}
        run = {'id':12,'head_branch':'develop','head_sha':'a'*40}
        d.validate_receipt(receipt,run)
        for field,value in [('source_ref','main'),('source_commit','main'),('workflow_run','13'),('image','ghcr.io/other/web:latest')]:
            with self.subTest(field=field), self.assertRaises(ValueError):
                d.validate_receipt(dict(receipt,**{field:value}),run)
        for field,value in [('head_branch','unreviewed'),('head_sha','c'*40)]:
            with self.subTest(field=field), self.assertRaisesRegex(ValueError,'publisher workflow source'):
                d.validate_receipt(receipt,dict(run,**{field:value}))

    def test_only_display_changes_are_admitted(self):
        d.validate_changes([{'filename':'web/src/pages/PapersPage.tsx'},{'filename':'.github/scripts/deploy_display.py'}])
        for filename in ['api/src/main.rs','infra/api/deployment.yaml','web/package.json','.github/workflows/ci.yml']:
            with self.subTest(filename=filename), self.assertRaisesRegex(ValueError,'outside display'):
                d.validate_changes([{'filename':filename}])
        with self.assertRaisesRegex(ValueError,'outside display'):
            d.validate_changes([{'filename':'web/src/moved.rs','previous_filename':'api/src/main.rs','status':'renamed'}])
        d.validate_changes([{'filename':'web/src/pages/New.tsx','previous_filename':'web/src/pages/Old.tsx','status':'renamed'}])

    def test_historical_manual_commit_receipt_is_accepted_only_as_the_baseline(self):
        receipt = {'source_ref':'run-'+'a'*12,'source_commit':'a'*40,'workflow_run':'12','image':d.IMAGE+'b'*64}
        run = {'id':12,'head_branch':'develop','head_sha':'a'*40}
        d.validate_receipt(receipt,run,baseline=True)
        with self.assertRaisesRegex(ValueError,'invalid source'):
            d.validate_receipt(receipt,run)
        with self.assertRaisesRegex(ValueError,'publisher workflow source'):
            d.validate_receipt(receipt,dict(run,head_sha='c'*40),baseline=True)

    def test_web_update_preserves_all_other_spec_values_and_the_original(self):
        before = {'project':'omega','name':'mathesis','owner_type':'org','owner':d.ORG,'spec':{'addons':{'mongodb':{'enabled':True}},'services':{
            'web':{'image':'old','image_digest':'computed','replicas':2},
            'worker':{'image':'unchanged','image_digest':'computed','env':{'PRIVATE':'preserved'}}}}}
        original = copy.deepcopy(before)
        after = d.desired_spec(before,'new')
        self.assertEqual(before,original)
        self.assertEqual(after['services']['web'],{'image':'new','replicas':2})
        self.assertEqual(after['services']['worker'],{'image':'unchanged','env':{'PRIVATE':'preserved'}})
        self.assertEqual(after['addons'],before['spec']['addons'])
        self.assertEqual(after['target_org_id'],d.ORG)
        with self.assertRaisesRegex(ValueError,'foreign CD application'):
            d.desired_spec(dict(before,owner='another-owner'),'new')

    def test_untrusted_or_changed_plan_never_contacts_deployment(self):
        with patch.object(d,'prepare',return_value={'changed':True}), patch.object(d,'cd') as cd:
            with self.assertRaisesRegex(ValueError,'plan changed'):
                d.apply({'image_run_id':'12'})
            cd.assert_not_called()

    def test_old_ready_pods_and_old_controller_receipts_cannot_confirm_rollout(self):
        observed = {'revision':7,'spec':{'services':{'web':{'replicas':2}}}}
        status = {'last_reconcile':{'ok':True,'revision':7},'services':{'web':{
            'ready':2,'updated':2,'pods':[{'ready':True,'app_container':{'image_id':'new'}} for _ in range(2)]}}}
        self.assertTrue(d.web_ready(status,observed,'new'))
        status['last_reconcile']['revision']=6
        self.assertFalse(d.web_ready(status,observed,'new'))
        status['last_reconcile']['revision']=7
        status['services']['web']['pods'][0]['app_container']['image_id']='old'
        self.assertFalse(d.web_ready(status,observed,'new'))

    def test_lost_update_receipt_is_not_retried(self):
        plan={'image_run_id':'12','source_commit':'b'*40,'image':d.IMAGE+'b'*64}
        before={'project':'omega','name':'mathesis','owner_type':'org','owner':d.ORG,'generation':3,'spec':{'services':{'web':{'image':d.IMAGE+'a'*64}}}}
        def cd(method='GET',*args,**kwargs):
            if method=='GET':return copy.deepcopy(before)
            raise TimeoutError('uncertain update')
        with patch.object(d,'prepare',return_value=plan), patch.object(d,'cd',side_effect=cd) as calls, \
                patch.object(d,'image_revision',return_value='a'*40), \
                patch.object(d,'github',return_value={'status':'ahead','files':[{'filename':'web/src/pages/Home.tsx'}]}):
            with self.assertRaises(TimeoutError):d.apply(plan,False)
            self.assertEqual(sum(c.args[0]=='PUT' for c in calls.call_args_list if c.args),1)
            self.assertEqual(calls.call_args.args[2],3)


if __name__ == '__main__':
    unittest.main()
