using System.Security.Cryptography;
using System.Text;

namespace JekyllPostTool.Domain.Posts;

/// <summary>
/// 博文全文内容哈希（SHA-256 十六进制），用于外部修改检测。
/// </summary>
public static class PostContentHash
{
    public static string Compute(string content) =>
        Convert.ToHexString(SHA256.HashData(Encoding.UTF8.GetBytes(content)));
}
