import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  reviewApi,
  type CreateReviewRequest,
  type MarketplaceStats,
  type ReviewResponse,
  type UpdateReviewRequest,
} from "../reviewApi";

// 辅助：构造模拟的 fetch Response
function mockFetchResponse(status: number, body: unknown) {
  return {
    ok: status >= 200 && status < 300,
    status,
    json: vi.fn().mockResolvedValue(body),
  };
}

// 辅助：创建示例 ReviewResponse
function makeReview(overrides: Partial<ReviewResponse> = {}): ReviewResponse {
  return {
    id: "rev-1",
    marketplace_id: "mp-1",
    user_id: "user-1",
    rating: 5,
    comment: "Great!",
    created_at: 1700000000000,
    ...overrides,
  };
}

describe("reviewApi", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    global.fetch = vi.fn() as unknown as typeof fetch;
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  // ═══════════════════════════════════════════════════════════════
  // getReviews
  // ═══════════════════════════════════════════════════════════════
  describe("getReviews", () => {
    it("应请求正确的 API 端点", async () => {
      const reviews = [makeReview()];
      (global.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce(
        mockFetchResponse(200, reviews),
      );

      await reviewApi.getReviews("mp-1");

      expect(global.fetch).toHaveBeenCalledWith("/api/marketplace/mp-1/reviews");
    });

    it("成功时应返回 ReviewResponse 数组", async () => {
      const reviews = [makeReview(), makeReview({ id: "rev-2" })];
      (global.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce(
        mockFetchResponse(200, reviews),
      );

      const result = await reviewApi.getReviews("mp-1");

      expect(result).toHaveLength(2);
      expect(result[0].id).toBe("rev-1");
    });

    it("非 200 响应应抛出错误", async () => {
      (global.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce(
        mockFetchResponse(500, { error: "Server error" }),
      );

      await expect(reviewApi.getReviews("mp-1")).rejects.toThrow("Server error");
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // getMyReview
  // ═══════════════════════════════════════════════════════════════
  describe("getMyReview", () => {
    it("应请求 /me 端点", async () => {
      (global.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce(
        mockFetchResponse(200, makeReview()),
      );

      await reviewApi.getMyReview("mp-1");

      expect(global.fetch).toHaveBeenCalledWith("/api/marketplace/mp-1/reviews/me");
    });

    it("404 时应返回 null", async () => {
      (global.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce(
        mockFetchResponse(404, {}),
      );

      const result = await reviewApi.getMyReview("mp-1");

      expect(result).toBeNull();
    });

    it("200 时应返回 ReviewResponse", async () => {
      const review = makeReview();
      (global.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce(
        mockFetchResponse(200, review),
      );

      const result = await reviewApi.getMyReview("mp-1");

      expect(result).toEqual(review);
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // getStats
  // ═══════════════════════════════════════════════════════════════
  describe("getStats", () => {
    it("应返回 MarketplaceStats", async () => {
      const stats: MarketplaceStats = {
        marketplace_id: "mp-1",
        total_reviews: 10,
        rating_average: 4.5,
      };
      (global.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce(
        mockFetchResponse(200, stats),
      );

      const result = await reviewApi.getStats("mp-1");

      expect(result.total_reviews).toBe(10);
      expect(result.rating_average).toBe(4.5);
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // createReview
  // ═══════════════════════════════════════════════════════════════
  describe("createReview", () => {
    it("应发送 POST 请求并携带 JSON body", async () => {
      const created = makeReview();
      (global.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce(
        mockFetchResponse(201, created),
      );

      const data: CreateReviewRequest = {
        marketplace_id: "mp-1",
        rating: 4,
        comment: "Nice",
      };
      const result = await reviewApi.createReview(data);

      expect(global.fetch).toHaveBeenCalledWith(
        "/api/marketplace/mp-1/reviews",
        expect.objectContaining({
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(data),
        }),
      );
      expect(result.id).toBe("rev-1");
    });

    it("不传 comment 时也应正常创建", async () => {
      const created = makeReview({ comment: null });
      (global.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce(
        mockFetchResponse(201, created),
      );

      await reviewApi.createReview({ marketplace_id: "mp-1", rating: 3 });

      const callArgs = (global.fetch as ReturnType<typeof vi.fn>).mock.calls[0];
      const body = JSON.parse(callArgs[1].body);
      expect(body.comment).toBeUndefined();
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // updateReview
  // ═══════════════════════════════════════════════════════════════
  describe("updateReview", () => {
    it("应发送 PATCH 请求", async () => {
      const updated = makeReview({ rating: 3 });
      (global.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce(
        mockFetchResponse(200, updated),
      );

      const data: UpdateReviewRequest = { rating: 3 };
      await reviewApi.updateReview("rev-1", data);

      expect(global.fetch).toHaveBeenCalledWith(
        "/api/reviews/rev-1",
        expect.objectContaining({ method: "PATCH" }),
      );
    });

    it("仅更新 comment", async () => {
      (global.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce(
        mockFetchResponse(200, makeReview()),
      );

      await reviewApi.updateReview("rev-1", { comment: "Updated" });

      const callArgs = (global.fetch as ReturnType<typeof vi.fn>).mock.calls[0];
      const body = JSON.parse(callArgs[1].body);
      expect(body.comment).toBe("Updated");
      expect(body.rating).toBeUndefined();
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // deleteReview
  // ═══════════════════════════════════════════════════════════════
  describe("deleteReview", () => {
    it("应发送 DELETE 请求", async () => {
      (global.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce(
        mockFetchResponse(204, null),
      );

      await reviewApi.deleteReview("rev-1");

      expect(global.fetch).toHaveBeenCalledWith(
        "/api/reviews/rev-1",
        expect.objectContaining({ method: "DELETE" }),
      );
    });

    it("删除成功不应返回值", async () => {
      (global.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce(
        mockFetchResponse(204, null),
      );

      const result = await reviewApi.deleteReview("rev-1");

      expect(result).toBeUndefined();
    });

    it("非 2xx 响应应抛出错误", async () => {
      (global.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce(
        mockFetchResponse(403, { error: "Forbidden" }),
      );

      await expect(reviewApi.deleteReview("rev-1")).rejects.toThrow("Forbidden");
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // 错误处理边界
  // ═══════════════════════════════════════════════════════════════
  describe("错误处理", () => {
    it("响应 json() 解析失败时应回退到通用错误", async () => {
      (global.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: false,
        status: 500,
        json: vi.fn().mockRejectedValue(new Error("Invalid JSON")),
      });

      await expect(reviewApi.getReviews("mp-1")).rejects.toThrow("HTTP 500");
    });

    it("网络请求失败应向上传播", async () => {
      (global.fetch as ReturnType<typeof vi.fn>).mockRejectedValueOnce(
        new Error("Network error"),
      );

      await expect(reviewApi.getReviews("mp-1")).rejects.toThrow("Network error");
    });
  });
});
