export interface UserModel {
  id: string;
  name: string;
  readonly email: string;
}
export interface PostModel {
  title: string;
  content: string | null;
  author_id: string;
}
