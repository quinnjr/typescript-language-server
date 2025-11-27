// Small TypeScript file for benchmarking (~50 lines)

interface User {
  id: number;
  name: string;
  email: string;
}

interface Post {
  id: number;
  title: string;
  content: string;
  author: User;
}

export function createUser(name: string, email: string): User {
  return {
    id: Math.floor(Math.random() * 1000),
    name,
    email,
  };
}

export function createPost(title: string, content: string, author: User): Post {
  return {
    id: Math.floor(Math.random() * 1000),
    title,
    content,
    author,
  };
}

export class UserService {
  private users: Map<number, User> = new Map();

  addUser(user: User): void {
    this.users.set(user.id, user);
  }

  getUser(id: number): User | undefined {
    return this.users.get(id);
  }

  getAllUsers(): User[] {
    return Array.from(this.users.values());
  }
}

const service = new UserService();
const user = createUser("John", "john@example.com");
service.addUser(user);

