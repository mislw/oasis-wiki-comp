import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
INTERACTION_GUIDE = (
    ROOT / 'references' / 'oasis-ui-agent-interaction.md'
).read_text(encoding='utf-8')
SKILL_GUIDE = (ROOT / 'SKILL.md').read_text(encoding='utf-8')
AGENT_GUIDE = (ROOT / 'AGENTS.md').read_text(encoding='utf-8')
TASK_ROUTER = (ROOT / 'references' / 'task-router.md').read_text(encoding='utf-8')
COWART_GUIDE = (
    ROOT / 'references' / 'cowart-ui-workflow.md'
).read_text(encoding='utf-8')


class OasisUiAgentInteractionTests(unittest.TestCase):
    def test_persisted_workflow_and_delivery_boundaries_are_explicit(self):
        markers = [
            '持久保存每个 UI 页面的八阶段任务',
            '完整 `load_path` 是唯一资产身份',
            '只调用 `ue_read` 和 `ue_py`',
            '最终重验',
            'one pending decision',
            'visual approval',
            '不是新的运行时状态模型',
            '不得把未持久化的聊天推断',
        ]
        for marker in markers:
            with self.subTest(marker=marker):
                self.assertIn(marker, INTERACTION_GUIDE)

    def test_required_review_and_write_gates_are_documented(self):
        markers = [
            'WORKBENCH_REVIEW',
            'UMG_REQUIREMENTS',
            'UMG_BUILD',
            'LOGIC_BINDING',
            'FINAL_REVIEW',
            'USER_APPROVAL_REQUIRED',
            'POSITION_PRESERVATION=PASS',
            '切图确认',
        ]
        for marker in markers:
            with self.subTest(marker=marker):
                self.assertIn(marker, INTERACTION_GUIDE)

    def test_modification_backtracking_and_stale_approval_rules_exist(self):
        markers = [
            '修改请求',
            'Backtracking',
            '旧的 layer approval 立即失效',
            'Visual 改动会使下游',
            '不得用“回到上一阶段”暗示自动回滚',
        ]
        for marker in markers:
            with self.subTest(marker=marker):
                self.assertIn(marker, INTERACTION_GUIDE)

    def test_skill_router_and_cowart_workflow_route_to_interaction_guide(self):
        marker = 'references/oasis-ui-agent-interaction.md'
        for guide in (SKILL_GUIDE, TASK_ROUTER, COWART_GUIDE):
            with self.subTest(guide_length=len(guide)):
                self.assertIn(marker, guide)

    def test_natural_language_ui_generation_requests_enter_text_source_guidance(self):
        trigger_examples = [
            '做一下 UI 生成',
            '我有一个 UI 需要生图',
            '帮我做个 UI',
            '启动 UI 生图工具',
        ]
        for trigger in trigger_examples:
            with self.subTest(trigger=trigger):
                self.assertIn(trigger, SKILL_GUIDE)
                self.assertIn(trigger, TASK_ROUTER)

        for marker in [
            '默认只进入当前对话的 SOURCE 文字引导',
            '还没有 UI，需要我先生成',
            '已经有 UI 图，直接使用',
            '继续之前做到一半的 UI',
            '不要等待用户再次说“开始”',
            '不得默认运行 `open_ui_workflow.py`',
        ]:
            with self.subTest(marker=marker):
                self.assertIn(marker, INTERACTION_GUIDE)

        for guide in (SKILL_GUIDE, AGENT_GUIDE):
            with self.subTest(guide_length=len(guide)):
                self.assertIn('默认进入 SOURCE 文字引导', guide)
                self.assertIn('不要自动打开 Companion', guide)

    def test_native_ui_workflow_opens_only_for_an_explicit_request(self):
        for guide in (SKILL_GUIDE, AGENT_GUIDE, INTERACTION_GUIDE):
            with self.subTest(guide_length=len(guide)):
                self.assertIn('打开原生 UI 工具链', guide)
                self.assertIn('open_ui_workflow.py', guide)


if __name__ == '__main__':
    unittest.main()
