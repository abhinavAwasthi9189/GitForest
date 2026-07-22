use octocrab;
use serde_json;
use std::env;
use tokio;



fn main()-> Result<(),Box<dyn std::error::Error>> {
    println!("give us the username");
    let username = strput();
    let gt = tokio::runtime::Runtime::new()?;
    println!("1");
    let _ = gt.block_on(async{get_git(&username).await}
    )?;
    println!("done!!!");
    Ok(())
}

//we have made a seprate async function so that everything async can be put in a seprate bucket
async fn get_git(username:&String)->Result<(),Box<dyn std::error::Error>>{
    rustls::crypto::ring::default_provider().install_default().unwrap();
    let octocrab = octocrab::Octocrab::builder().personal_token(env::var("GITHUB_TOKEN")?).build()?;

    println!("2");
    //here we get a list of all the repos that the user has
    let repos = octocrab.users(username.clone()).repos().send().await?;
    println!("3");
    for getrepo in repos{
        println!("first of all for {}",getrepo.name);
        let repo = octocrab.repos(username.clone(),getrepo.name).list_commits().author(username.clone()).send().await?;
        //here we take the repo from github and then go through the commits
        for commitset in repo{
            //if we have an author and date for the commit. we print it.
            if let Some(commit) = &commitset.commit.author{
                if let Some(date) = &commit.date{
                    println!("{}--->{}",date,commitset.commit.message);
                }
            }
        }

        print!("\n\n");
    }   
    Ok(())

}


fn strput() -> String {
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).expect("Failed to read line");
    input.trim().to_string()
}
