"""
AxAgent Python SDK — ACP 协议客户端

通过 ACP 协议与 AxAgent 交互：
    from axagent_sdk import AxAgentClient

    client = AxAgentClient("http://localhost:9876")
    session = client.create_session(work_dir="/path/to/project")
    result = client.send_prompt(session["sessionId"], "分析项目结构")
    print(result["content"])
    client.close_session(session["sessionId"])
"""

import json
import time
import urllib.request
import urllib.error
from typing import Optional, List, Dict, Any, Iterator


class AxAgentClient:
    """ACP 协议 HTTP 客户端"""

    def __init__(
        self,
        base_url: str,
        auth_token: Optional[str] = None,
        timeout: float = 30.0,
        max_retries: int = 3,
        retry_backoff: float = 0.5,
    ):
        self.base_url = base_url.rstrip("/")
        self.auth_token = auth_token
        self.timeout = timeout
        self.max_retries = max_retries
        self.retry_backoff = retry_backoff

    def _headers(self) -> Dict[str, str]:
        h = {"Content-Type": "application/json"}
        if self.auth_token:
            h["Authorization"] = f"Bearer {self.auth_token}"
        return h

    def _request(self, method: str, path: str, body: Any = None) -> Any:
        url = f"{self.base_url}{path}"
        data = json.dumps(body).encode("utf-8") if body else None
        req = urllib.request.Request(url, data=data, headers=self._headers(), method=method)
        last_err: Optional[Exception] = None
        for attempt in range(self.max_retries):
            try:
                with urllib.request.urlopen(req, timeout=self.timeout) as resp:
                    raw = resp.read().decode("utf-8")
                    try:
                        return json.loads(raw)
                    except json.JSONDecodeError as e:
                        # 响应体非合法 JSON —— 不重试，包装后抛出
                        raise Exception(f"ACP 响应 JSON 解析失败: {e} (body={raw[:200]})") from e
            except urllib.error.HTTPError as e:
                # 4xx/5xx 业务错误 —— 解析错误体，不重试
                error_body = e.read().decode("utf-8")
                try:
                    error_data = json.loads(error_body)
                    raise Exception(error_data.get("error", {}).get("message", str(e)))
                except json.JSONDecodeError:
                    raise Exception(f"ACP 请求失败: {e.code} {error_body}")
            except (urllib.error.URLError, TimeoutError) as e:
                # 网络层错误（含连接失败/超时，urlopen 将 socket.timeout 包装为 URLError）—— 可重试
                last_err = e
                if attempt < self.max_retries - 1:
                    time.sleep(self.retry_backoff * (attempt + 1))
                    continue
                raise Exception(
                    f"ACP 请求失败（重试 {self.max_retries} 次仍失败）: {e}"
                ) from e
        if last_err is not None:
            raise last_err
        return None

    def create_session(self, work_dir: str, model: Optional[str] = None,
                       permission_mode: Optional[str] = None,
                       system_prompt: Optional[str] = None) -> Dict[str, Any]:
        """创建新会话"""
        params = {"work_dir": work_dir}
        if model:
            params["model"] = model
        if permission_mode:
            params["permission_mode"] = permission_mode
        if system_prompt:
            params["system_prompt"] = system_prompt
        return self._request("POST", "/acp/v1/sessions", params)

    def get_session(self, session_id: str) -> Dict[str, Any]:
        """获取会话状态"""
        return self._request("GET", f"/acp/v1/sessions/{session_id}")

    def list_sessions(self) -> List[Dict[str, Any]]:
        """列出所有会话"""
        return self._request("GET", "/acp/v1/sessions")

    def send_prompt(self, session_id: str, prompt: str,
                    max_turns: Optional[int] = None) -> Dict[str, Any]:
        """发送 prompt 并获取响应"""
        params = {"session_id": session_id, "prompt": prompt}
        if max_turns:
            params["max_turns"] = max_turns
        return self._request("POST", f"/acp/v1/sessions/{session_id}/prompts", params)

    def interrupt(self, session_id: str) -> None:
        """中断执行"""
        self._request("POST", f"/acp/v1/sessions/{session_id}/interrupt")

    def close_session(self, session_id: str) -> None:
        """关闭会话"""
        self._request("POST", f"/acp/v1/sessions/{session_id}/close")

    def register_hook(self, session_id: str, event: str, callback_url: str) -> None:
        """注册 hook 回调"""
        self._request("POST", "/acp/v1/hooks", {
            "session_id": session_id,
            "event": event,
            "callback_url": callback_url,
        })

    def health_check(self) -> bool:
        """健康检查"""
        try:
            self.get_session("health")
            return True
        except Exception:
            return False


class AxAgentSession:
    """会话上下文管理器"""

    def __init__(self, client: AxAgentClient, work_dir: str, **kwargs):
        self.client = client
        self.work_dir = work_dir
        self.kwargs = kwargs
        self.session = None

    def __enter__(self):
        self.session = self.client.create_session(self.work_dir, **self.kwargs)
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        if self.session:
            self.client.close_session(self.session["sessionId"])

    def send(self, prompt: str) -> Dict[str, Any]:
        return self.client.send_prompt(self.session["sessionId"], prompt)

    @property
    def session_id(self) -> str:
        return self.session["sessionId"] if self.session else ""
