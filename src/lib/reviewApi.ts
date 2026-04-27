const API_BASE = "/api";

export interface ReviewResponse {
  id: string;
  marketplace_id: string;
  user_id: string;
  rating: number;
  comment: string | null;
  created_at: number;
}

export interface MarketplaceStats {
  marketplace_id: string;
  total_reviews: number;
  rating_average: number;
}

export interface CreateReviewRequest {
  marketplace_id: string;
  rating: number;
  comment?: string;
}

export interface UpdateReviewRequest {
  rating?: number;
  comment?: string;
}

async function handleResponse<T>(response: Response): Promise<T> {
  if (!response.ok) {
    const error = await response.json().catch(() => ({ error: "Unknown error" }));
    throw new Error(error.error || `HTTP ${response.status}`);
  }
  return response.json();
}

export const reviewApi = {
  async getReviews(marketplaceId: string): Promise<ReviewResponse[]> {
    const response = await fetch(`${API_BASE}/marketplace/${marketplaceId}/reviews`);
    return handleResponse(response);
  },

  async getMyReview(marketplaceId: string): Promise<ReviewResponse | null> {
    const response = await fetch(`${API_BASE}/marketplace/${marketplaceId}/reviews/me`);
    if (response.status === 404) return null;
    return handleResponse(response);
  },

  async getStats(marketplaceId: string): Promise<MarketplaceStats> {
    const response = await fetch(`${API_BASE}/marketplace/${marketplaceId}/stats`);
    return handleResponse(response);
  },

  async createReview(data: CreateReviewRequest): Promise<ReviewResponse> {
    const response = await fetch(`${API_BASE}/marketplace/${data.marketplace_id}/reviews`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(data),
    });
    return handleResponse(response);
  },

  async updateReview(reviewId: string, data: UpdateReviewRequest): Promise<ReviewResponse> {
    const response = await fetch(`${API_BASE}/reviews/${reviewId}`, {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(data),
    });
    return handleResponse(response);
  },

  async deleteReview(reviewId: string): Promise<void> {
    const response = await fetch(`${API_BASE}/reviews/${reviewId}`, {
      method: "DELETE",
    });
    if (!response.ok) {
      const error = await response.json().catch(() => ({ error: "Unknown error" }));
      throw new Error(error.error || `HTTP ${response.status}`);
    }
  },
};
