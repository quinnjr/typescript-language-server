// Medium TypeScript file for benchmarking (~300 lines)

import type { ReactNode } from 'react';

// Types and interfaces
interface BaseEntity {
  id: string;
  createdAt: Date;
  updatedAt: Date;
}

interface User extends BaseEntity {
  username: string;
  email: string;
  passwordHash: string;
  role: UserRole;
  profile: UserProfile;
}

interface UserProfile {
  firstName: string;
  lastName: string;
  avatar?: string;
  bio?: string;
}

type UserRole = 'admin' | 'moderator' | 'user' | 'guest';

interface Post extends BaseEntity {
  title: string;
  content: string;
  authorId: string;
  tags: string[];
  status: PostStatus;
  metadata: PostMetadata;
}

interface PostMetadata {
  views: number;
  likes: number;
  shares: number;
}

type PostStatus = 'draft' | 'published' | 'archived';

interface Comment extends BaseEntity {
  postId: string;
  authorId: string;
  content: string;
  parentId?: string;
}

// Utility types
type CreateInput<T extends BaseEntity> = Omit<T, 'id' | 'createdAt' | 'updatedAt'>;
type UpdateInput<T extends BaseEntity> = Partial<Omit<T, 'id' | 'createdAt' | 'updatedAt'>>;

// Generic repository pattern
interface Repository<T extends BaseEntity> {
  findById(id: string): Promise<T | null>;
  findAll(): Promise<T[]>;
  create(data: CreateInput<T>): Promise<T>;
  update(id: string, data: UpdateInput<T>): Promise<T | null>;
  delete(id: string): Promise<boolean>;
}

// Base repository implementation
abstract class BaseRepository<T extends BaseEntity> implements Repository<T> {
  protected items: Map<string, T> = new Map();

  async findById(id: string): Promise<T | null> {
    return this.items.get(id) ?? null;
  }

  async findAll(): Promise<T[]> {
    return Array.from(this.items.values());
  }

  abstract create(data: CreateInput<T>): Promise<T>;

  async update(id: string, data: UpdateInput<T>): Promise<T | null> {
    const existing = this.items.get(id);
    if (!existing) return null;

    const updated = {
      ...existing,
      ...data,
      updatedAt: new Date(),
    } as T;

    this.items.set(id, updated);
    return updated;
  }

  async delete(id: string): Promise<boolean> {
    return this.items.delete(id);
  }

  protected generateId(): string {
    return Math.random().toString(36).substring(2, 15);
  }
}

// User repository
class UserRepository extends BaseRepository<User> {
  async create(data: CreateInput<User>): Promise<User> {
    const user: User = {
      ...data,
      id: this.generateId(),
      createdAt: new Date(),
      updatedAt: new Date(),
    };
    this.items.set(user.id, user);
    return user;
  }

  async findByEmail(email: string): Promise<User | null> {
    for (const user of this.items.values()) {
      if (user.email === email) {
        return user;
      }
    }
    return null;
  }

  async findByRole(role: UserRole): Promise<User[]> {
    return Array.from(this.items.values()).filter(u => u.role === role);
  }
}

// Post repository
class PostRepository extends BaseRepository<Post> {
  async create(data: CreateInput<Post>): Promise<Post> {
    const post: Post = {
      ...data,
      id: this.generateId(),
      createdAt: new Date(),
      updatedAt: new Date(),
    };
    this.items.set(post.id, post);
    return post;
  }

  async findByAuthor(authorId: string): Promise<Post[]> {
    return Array.from(this.items.values()).filter(p => p.authorId === authorId);
  }

  async findByStatus(status: PostStatus): Promise<Post[]> {
    return Array.from(this.items.values()).filter(p => p.status === status);
  }

  async findByTag(tag: string): Promise<Post[]> {
    return Array.from(this.items.values()).filter(p => p.tags.includes(tag));
  }
}

// Service layer
class UserService {
  constructor(private readonly userRepo: UserRepository) {}

  async registerUser(
    username: string,
    email: string,
    password: string
  ): Promise<User> {
    const existing = await this.userRepo.findByEmail(email);
    if (existing) {
      throw new Error('Email already registered');
    }

    return this.userRepo.create({
      username,
      email,
      passwordHash: this.hashPassword(password),
      role: 'user',
      profile: {
        firstName: '',
        lastName: '',
      },
    });
  }

  async updateProfile(
    userId: string,
    profile: Partial<UserProfile>
  ): Promise<User | null> {
    const user = await this.userRepo.findById(userId);
    if (!user) return null;

    return this.userRepo.update(userId, {
      profile: { ...user.profile, ...profile },
    });
  }

  private hashPassword(password: string): string {
    // Simplified - would use bcrypt in production
    return Buffer.from(password).toString('base64');
  }
}

class PostService {
  constructor(
    private readonly postRepo: PostRepository,
    private readonly userRepo: UserRepository
  ) {}

  async createPost(
    authorId: string,
    title: string,
    content: string,
    tags: string[] = []
  ): Promise<Post> {
    const author = await this.userRepo.findById(authorId);
    if (!author) {
      throw new Error('Author not found');
    }

    return this.postRepo.create({
      title,
      content,
      authorId,
      tags,
      status: 'draft',
      metadata: { views: 0, likes: 0, shares: 0 },
    });
  }

  async publishPost(postId: string): Promise<Post | null> {
    return this.postRepo.update(postId, { status: 'published' });
  }

  async incrementViews(postId: string): Promise<void> {
    const post = await this.postRepo.findById(postId);
    if (post) {
      await this.postRepo.update(postId, {
        metadata: { ...post.metadata, views: post.metadata.views + 1 },
      });
    }
  }

  async getPopularPosts(limit: number = 10): Promise<Post[]> {
    const published = await this.postRepo.findByStatus('published');
    return published
      .sort((a, b) => b.metadata.views - a.metadata.views)
      .slice(0, limit);
  }
}

// Helper functions
function formatDate(date: Date): string {
  return date.toISOString().split('T')[0];
}

function slugify(text: string): string {
  return text
    .toLowerCase()
    .replace(/[^\w\s-]/g, '')
    .replace(/\s+/g, '-');
}

function truncate(text: string, maxLength: number): string {
  if (text.length <= maxLength) return text;
  return text.slice(0, maxLength - 3) + '...';
}

// Export everything
export {
  User,
  Post,
  Comment,
  UserRole,
  PostStatus,
  UserRepository,
  PostRepository,
  UserService,
  PostService,
  formatDate,
  slugify,
  truncate,
};

